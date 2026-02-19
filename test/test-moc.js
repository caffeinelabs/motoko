process.on("unhandledRejection", (error) => {
  console.log(`Unhandled promise rejection:\n${error}`)
});

process.on("uncaughtException", (error) => {
  console.log(`Uncaught exception:\n${error}`);
})

const assert = require("assert").strict;

// Load moc.js
const { Motoko } = require("moc.js");

// Store files
Motoko.saveFile("empty.mo", "");
Motoko.saveFile("ok.mo", "1");
Motoko.saveFile("bad.mo", "1+");
Motoko.saveFile(
  "actor.mo",
  'persistent actor { type A<B> = B; public query func main() : async A<Text> { "abc" } }'
);
Motoko.saveFile(
  "ast.mo",
  `
  /** Program comment
      multi-line */
  import Prim "mo:prim";

  persistent actor {
    /// Type comment
    type T = Nat;
    /// Variable comment
    var x : T = 0;
    /** Function comment */
    public query func main() : async T { x };
    /// Sub-module comment
    module M {
      /// Class comment
      public class C() {};
    };
  }`
);
Motoko.saveFile("text.mo", `let s = "${"⛔|".repeat(10000)}"; s.size()`); // #3822

assert.equal(Motoko.readFile("empty.mo"), "");
assert.equal(Motoko.readFile("ok.mo"), "1");

// Compile the empty module in wasi and ic mode
const empty_wasm_plain = Motoko.compileWasm("wasi", "empty.mo");
const empty_wasm_ic = Motoko.compileWasm("ic", "empty.mo");

// For the plain module...
// Check that the code looks like a WebAssembly binary
assert.equal(typeof empty_wasm_plain, "object");
assert.deepEqual(
  empty_wasm_plain.code.wasm.subarray(0, 4),
  new Uint8Array([0, 97, 115, 109])
);
assert.deepEqual(
  empty_wasm_plain.code.wasm.subarray(4, 8),
  new Uint8Array([1, 0, 0, 0])
);
assert.equal(typeof empty_wasm_plain.diagnostics, "object");
assert.equal(empty_wasm_plain.diagnostics.length, 0);

// Check that the WebAssembly binary can be loaded
WebAssembly.compile(empty_wasm_plain.code.wasm);

// Now again for the ic module
assert.equal(typeof empty_wasm_ic, "object");
assert.deepEqual(
  empty_wasm_plain.code.wasm.subarray(0, 4),
  new Uint8Array([0, 97, 115, 109])
);
assert.deepEqual(
  empty_wasm_plain.code.wasm.subarray(4, 8),
  new Uint8Array([1, 0, 0, 0])
);
assert.equal(typeof empty_wasm_ic.diagnostics, "object");
assert.equal(empty_wasm_ic.diagnostics.length, 0);

WebAssembly.compile(empty_wasm_ic.code.wasm);

// The plain and the ic module should not be the same
assert.notEqual(empty_wasm_plain.code.wasm, empty_wasm_ic.code.wasm);

Motoko.removeFile("empty.mo");
assert.throws(() => {
  Motoko.compileWasm("ic", "empty.mo");
}, /No such file or directory/);

// Check if error messages are correctly returned
const bad_result = Motoko.compileWasm("ic", "bad.mo");
// Uncomment to see what to paste below
// console.log(JSON.stringify(bad_result, null, 2));
assert.deepStrictEqual(bad_result, {
  diagnostics: [
    {
      range: {
        start: {
          line: 0,
          character: 2,
        },
        end: {
          line: 0,
          character: 2,
        },
      },
      severity: 1,
      source: "bad.mo",
      code: "M0001",
      category: "syntax",
      message:
        "unexpected end of input, expected one of token or <phrase> sequence:\n  <exp_bin(ob)> (e.g. '42')",
    },
  ],
  code: null,
});

// Check the check command (should print errors, but have no code)
assert.deepStrictEqual(Motoko.check("ok.mo"), {
  diagnostics: [],
  code: null,
});

assert.deepStrictEqual(Motoko.check("bad.mo"), {
  diagnostics: [
    {
      range: {
        start: {
          line: 0,
          character: 2,
        },
        end: {
          line: 0,
          character: 2,
        },
      },
      severity: 1,
      source: "bad.mo",
      category: "syntax",
      code: "M0001",
      message:
        "unexpected end of input, expected one of token or <phrase> sequence:\n  <exp_bin(ob)> (e.g. '42')",
    },
  ],
  code: null,
});

// Run interpreter
assert.deepStrictEqual(Motoko.run([], "actor.mo"), {
  stdout: "`ys6dh-5cjiq-5dc` : actor {main : shared query () -> async A<Text>}\n",
  stderr: "",
  result: { error: null },
});

// Check AST format
const astFile = Motoko.readFile("ast.mo");
for (const ast of [
  Motoko.parseMotoko(/*enable_recovery=*/false, astFile),
  Motoko.parseMotokoTyped(["ast.mo"]).code[0].ast, // { diagnostics; code: [{ ast; scope }] }
  Motoko.parseMotokoTypedWithScopeCache(/*enable_recovery=*/false, ["ast.mo"], new Map()).code[0][0].ast, // { diagnostics; code: [[{ ast; immediateImports; scope }], cache] }
  Motoko.parseMotoko(/*enable_recovery=*/true, astFile),
  Motoko.parseMotokoTypedWithScopeCache(/*enable_recovery=*/true, ["ast.mo"], new Map()).code[0][0].ast, // { diagnostics; code: [[{ ast; immediateImports; scope }], cache] }
]) {
  const astString = JSON.stringify(ast);

  // Check doc comments
  assert.match(
    astString,
    /"name":"\*","args":\["Program comment\\n      multi-line"/
  );
  assert.match(astString, /"name":"\*","args":\["Type comment"/);
  assert.match(astString, /"name":"\*","args":\["Variable comment"/);
  assert.match(astString, /"name":"\*","args":\["Function comment"/);
  assert.match(astString, /"name":"\*","args":\["Sub-module comment"/);
  assert.match(astString, /"name":"\*","args":\["Class comment"/);
}

// Check that long text literals type-check without error
assert.deepStrictEqual(Motoko.check("text.mo"), {
  code: null,
  diagnostics: [],
});

// Check Candid format
const candid =
  `
type T = nat;
/// Program comment
///       multi-line
service : {
  /// Function comment
  main: () -> (T) query;
}
`.trim() + "\n";
assert.deepStrictEqual(Motoko.candid("ast.mo").code, candid)

// Check that parseMotokoTyped exposes scope
const typedResult = Motoko.parseMotokoTyped(["ast.mo"]);
assert(typedResult.code[0].scope != null);

// Check that parseMotokoTypedWithScopeCache exposes scope
const typedWithCacheResult = Motoko.parseMotokoTypedWithScopeCache(/*enable_recovery=*/true, ["ast.mo"], new Map());
assert(typedWithCacheResult.code[0][0].scope != null);

// Test contextualDotSuggestions and contextualDotModule
Motoko.saveFile(
  "lib.mo",
  'module { public func foo(self : Text) { ignore self }; public func bar(self : Text, n : Nat) : Nat { n } }'
);
Motoko.saveFile(
  "dot.mo",
  'import Lib "lib"; let t = "world"; t.foo()'
);
const dotResult = Motoko.parseMotokoTypedWithScopeCache(
  /*enable_recovery=*/true, ["dot.mo"], new Map()
);
assert(dotResult.code != null, "dot.mo should parse successfully");
const dotAst = dotResult.code[0][0].ast;
const dotScope = dotResult.code[0][0].scope;

// Navigate: Prog -> last decl(@) -> ExpD -> call(@:) -> CallE -> DotE(@:)
const expD = dotAst.args[2].args[2];
assert.equal(expD.name, "ExpD");
const callE = expD.args[0].args[2].args[0];
assert.equal(callE.name, "CallE");
const dotE = callE.args[0].args[2].args[0];
assert.equal(dotE.name, "DotE");

// contextualDotModule: t.foo resolves to the lib module
assert(dotE.rawExp != null);
const dotModule = Motoko.contextualDotModule(dotE.rawExp);
assert(dotModule != null, "contextualDotModule should resolve for contextual dot");
assert.equal(dotModule.funcName, "foo");
assert.equal(dotModule.moduleNameOrUri, "Lib");

// contextualDotModule returns null for non-DotE expressions
const varE = dotE.args[0].args[2].args[0];
assert.equal(varE.name, "VarE");
assert.equal(Motoko.contextualDotModule(varE.rawExp), null);

// contextualDotSuggestions: receiver of DotE (VarE "t" : Text) should suggest foo
const suggestions = Motoko.contextualDotSuggestions(dotScope, varE.rawExp);
assert.deepStrictEqual(suggestions, [
  { moduleUri: "lib.mo", funcName: "bar", funcType: "(self : Text, n : Nat) -> Nat" },
  { moduleUri: "lib.mo", funcName: "foo", funcType: "(self : Text) -> ()" },
]);

// Check error recovery
const badAstFile = Motoko.readFile("bad.mo");

assert(Motoko.parseMotoko(/*enable_recovery=*/false, badAstFile).code == null);
assert(Motoko.parseMotoko(/*enable_recovery=*/true, badAstFile).code != null);
assert(Motoko.parseMotokoTypedWithScopeCache(/*enable_recovery=*/false, ["bad.mo"], new Map()).code == null);

// TODO: This requires avoid dropping 'code' field in all checks though all pipeline e.g. infer_prog
// assert(Motoko.parseMotokoTypedWithScopeCache(/*enable_recovery=*/true, ["bad.mo"], new Map()).code != null);

// `blob:` import placeholders
Motoko.setBlobImportPlaceholders(true);
Motoko.saveFile("blob.mo", 'import MyBlob "blob:file:path/to/blob.txt"; MyBlob.size();');
assert(Motoko.parseMotoko(/*enable_recovery=*/true, "blob.mo").code != null);
assert.deepStrictEqual(Motoko.run([], "blob.mo"), {
  stdout: "",
  stderr: "blob.mo:1.1-1.43: execution error, blob import placeholder\n",
  result: { error: {} },
});

Motoko.setExtraFlags(["-W=M0223"]);
assert.throws(
  () => Motoko.setExtraFlags(["--invalid-flag"]),
  /unknown option/
);
assert.throws(
  () => Motoko.setExtraFlags(["-W=MMM"]),
  /moc: invalid warning code: MMM/
);
