import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PocketIc, PocketIcServer } from '@dfinity/pic';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '../..');
const cache = resolve(here, '.cache');
mkdirSync(cache, { recursive: true });
const originalPath = resolve(cache, 'moc-2.0.0.cjs');
const patchedPath = resolve(cache, 'moc-patched.cjs');
if (!existsSync(originalPath)) {
  console.log('Downloading the official moc 2.0.0 JavaScript compiler...');
  execFileSync('curl', ['-fL', '--retry', '2', '--max-time', '180', '-o', originalPath,
    'https://github.com/caffeinelabs/motoko/releases/download/2.0.0/moc-2.0.0.js'],
  { stdio: 'inherit' });
}
const compiler = readFileSync(originalPath);
assert.equal(createHash('sha256').update(compiler).digest('hex'),
  'd20a6eb998543f6c8ca5fa93448d7ff7e360106e28337e339d09522764cab572',
  'Unexpected compiler download; remove .cache/moc-2.0.0.cjs and retry');
// Forks need not contain upstream release tags or release-branch commits.
// Fetch only the original source file, pinned to the release commit.
const releaseCommit = '1bd3011186ed6ee3d12c12735acb9ffd1e6f475b';
const originalPreludePath = resolve(cache, 'internals-2.0.0.mo');
if (!existsSync(originalPreludePath)) {
  execFileSync('curl', ['-fL', '--retry', '2', '--max-time', '180', '-o', originalPreludePath,
    `https://raw.githubusercontent.com/caffeinelabs/motoko/${releaseCommit}/src/prelude/internals.mo`],
  { stdio: 'inherit' });
}
const original = readFileSync(originalPreludePath, 'utf8');
assert.equal(createHash('sha256').update(original).digest('hex'),
  '69401d3d2c95913f55a29ea570215d484fc6a21fea135858eb2a8c6204d5ca47',
  'Unexpected Prelude download; remove .cache/internals-2.0.0.mo and retry');
const patched = readFileSync(resolve(repo, 'src/prelude/internals.mo'), 'utf8');
function section(source) {
  const start = source.indexOf('func @new_async');
  const end = source.indexOf('// Subset of IC management', start);
  assert(start >= 0 && end > start, 'Future implementation boundaries not found');
  return source.slice(start, end);
}
const escape = value => JSON.stringify(value).slice(1, -1);
const oldSection = escape(section(original));
assert.equal(compiler.toString().split(oldSection).length, 2,
  'Expected exactly one embedded future implementation');
assert.notEqual(section(original), section(patched), 'The Prelude fix is missing');
writeFileSync(patchedPath, compiler.toString().replace(oldSection, () => escape(section(patched))));

const require = createRequire(import.meta.url);
const source = readFileSync(resolve(repo, 'test/run-drun/await-trapped/await-trapped.mo'), 'utf8');
const modules = {};
for (const [name, path] of [['baseline', originalPath], ['patched', patchedPath]]) {
  const { Motoko } = require(path);
  Motoko.saveFile('regression.mo', source);
  const result = Motoko.compileWasm('ic', 'regression.mo');
  assert.deepEqual(result.diagnostics, [], `${name}: compiler diagnostics`);
  assert(result.code, `${name}: no Wasm generated`);
  modules[name] = result.code.wasm;
  writeFileSync(resolve(cache, `${name}.wasm`), result.code.wasm);
  console.log(`${name}: compiled ${result.code.wasm.length} bytes`);
}

// Use precisely the submitted calls, rounds, and checks from the CI fixture.
// Never await the response to submit: termination is checked through queries.
const commands = readFileSync(resolve(repo, 'test/run-drun/await-trapped.drun'), 'utf8')
  .trim().split('\n');
const emptyArgs = Uint8Array.from([68, 73, 68, 76, 0, 0]);
async function runFixture(url, name, wasm) {
  const pic = await PocketIc.create(url);
  let failedChecks = 0;
  let checks = 0;
  try {
    const canisterId = await pic.createCanister();
    await pic.installCode({ canisterId, wasm });
    for (const line of commands) {
      if (line === 'install $ID await-trapped/await-trapped.mo ""') continue;
      const tick = /^tick (\d+)$/.exec(line);
      if (tick) { await pic.tick(Number(tick[1])); continue; }
      const call = /^(submit|query|ingress) \$ID (\w+) "DIDL\\x00\\x00"$/.exec(line);
      assert(call, `Unsupported fixture command: ${line}`);
      const [, kind, method] = call;
      const options = { canisterId, method, arg: emptyArgs };
      if (kind === 'submit') {
        await pic.submitCall(options);
      } else if (kind === 'ingress') {
        assert.deepEqual(await pic.updateCall(options), emptyArgs);
        console.log(`${name}: ${method} PASS`);
      } else {
        checks += 1;
        let result;
        try {
          result = await pic.queryCall(options);
        } catch (error) {
          // Only canister assertion failures count as reproduction. Network,
          // installation, timeout, and other failures must fail the runner.
          if (name !== 'baseline' || !/assertion fail/i.test(String(error))) throw error;
          failedChecks += 1;
          console.log(`${name}: check ${checks} failed as expected`);
          continue;
        }
        assert.deepEqual(result, emptyArgs);
        console.log(`${name}: check ${checks} PASS`);
      }
    }
    assert.equal(checks, 4, 'The fixture changed; review the expected checks');
    assert.equal(failedChecks, name === 'baseline' ? 4 : 0,
      `${name}: unexpected regression outcome`);
  } finally {
    await pic.tearDown();
  }
}

if (!process.argv.includes('--compile-only')) {
  // POCKET_IC_URL reuses an already running local server. Otherwise the
  // package starts its downloaded binary (or POCKET_IC_BIN, when set).
  const server = process.env.POCKET_IC_URL ? null : await PocketIcServer.start();
  try {
    const url = process.env.POCKET_IC_URL ?? server.getUrl();
    await runFixture(url, 'baseline', modules.baseline);
    await runFixture(url, 'patched', modules.patched);
    console.log('PASS: baseline reproduces the bug; patched Prelude passes all checks.');
  } finally {
    if (server) await server.stop();
  }
}
