# Bumping pocket-ic-server

This skill guides you through bumping the `pocket-ic-server` binary used in
Motoko's test suite. The binary is downloaded directly from dfinity/ic GitHub
releases — it is NOT tracked via a flake input.

## File to Change

`nix/pocket-ic.nix` — update `releaseTag` and `sha256Map`.

## Step-by-Step

### 1. Pick a release tag

Browse https://github.com/dfinity/ic/releases and pick a release tagged
`release-YYYY-MM-DD_HH-MM-base`. Update `releaseTag` in `nix/pocket-ic.nix`.

### 2. Get the SHA256 hashes

**Option A — from the GitHub release page (recommended for verification):**
The release page lists checksums. Look for or fetch `SHA256SUMS`:
```sh
curl -sL "https://github.com/dfinity/ic/releases/download/<tag>/SHA256SUMS" | grep "pocket-ic"
```
These are standard hex SHA256 hashes of the `.gz` files.

**Option B — via nix-prefetch-url (cross-check):**
```sh
for name in pocket-ic-x86_64-linux pocket-ic-arm64-linux pocket-ic-x86_64-darwin pocket-ic-arm64-darwin; do
  hash=$(nix-prefetch-url --print-path \
    "https://github.com/dfinity/ic/releases/download/<tag>/${name}.gz" 2>/dev/null | head -1)
  hex=$(nix hash to-base16 --type sha256 "$hash")
  echo "$name: $hex"
done
```
**Always compare Option A and Option B — they must match exactly.**

### 3. Update `nix/pocket-ic.nix`

```nix
releaseTag = "release-YYYY-MM-DD_HH-MM-base";
sha256Map = {
  "pocket-ic-x86_64-linux" = "sha256:<hex>";
  "pocket-ic-arm64-linux"  = "sha256:<hex>";
  "pocket-ic-x86_64-darwin" = "sha256:<hex>";
  "pocket-ic-arm64-darwin"  = "sha256:<hex>";
};
```

### 4. Test locally (optional)
```sh
nix build .#pocket-ic-server
```

### 5. Bump the `test-runner` Rust crate

The `test-runner/Cargo.toml` pins the `pocket-ic` Rust crate to the same
release tag. Update it to match:

```toml
pocket-ic = { git = "https://github.com/dfinity/ic", tag = "release-YYYY-MM-DD_HH-MM-base", package = "pocket-ic" }
```

Then regenerate the lockfile:
```sh
cd test-runner && cargo update
```

### 6. Fix the `flake.nix` output hash

`flake.nix` contains a `test-runner-cargo-lock` block that pins the
`pocket-ic-<version>` crate hash. After bumping the tag the version number
may change (e.g. `pocket-ic-12.0.0` → `pocket-ic-13.0.0`). Read the new
version from `test-runner/Cargo.lock` (the `version` field under
`name = "pocket-ic"`), then update both the key and the hash in `flake.nix`:

```nix
test-runner-cargo-lock = {
  lockFile = ./test-runner/Cargo.lock;
  outputHashes = {
    "pocket-ic-X.Y.Z" = "sha256-<base64>";
  };
};
```

To obtain the correct hash, either:
- Set a dummy hash (`sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=`),
  run `nix build`, and paste the `got:` value back in; or
- Let the first `nix build` failure tell you the expected hash.

### 7. Test locally (optional)
```sh
nix build .#pocket-ic-server .#test-runner
```

### 8. Commit (three separate commits keeps the history readable)
```
bump `pocket-ic`            ← nix/pocket-ic.nix only
bump dependencies           ← test-runner/Cargo.toml + Cargo.lock
fix hash in flake           ← flake.nix outputHashes
```

## Notes
- Pocket-ic releases weekly; not every release is stable. Prefer releases that
  have been out for at least a week with no reported breakage.
- The server binary version and the Rust crate version must be kept in sync —
  both are pinned to the same `releaseTag`.
- The four binary names map to Nix systems:
  - `x86_64-linux` → `pocket-ic-x86_64-linux`
  - `aarch64-linux` → `pocket-ic-arm64-linux`
  - `x86_64-darwin` → `pocket-ic-x86_64-darwin`
  - `aarch64-darwin` → `pocket-ic-arm64-darwin`
