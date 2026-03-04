- [Nix setup](#nix-setup)
- [Installation using Nix](#installation-using-nix)
  - [Other tools](#other-tools)
- [Development using Nix](#development-using-nix)
- [Replicating CI locally](#replicating-ci-locally)
- [Making releases](#making-releases)
  - [1. Update Changelog](#1-update-changelog)
  - [2. Open a release PR](#2-open-a-release-pr)
  - [3. Wait for the release to complete, and verify it](#3-wait-for-the-release-to-complete-and-verify-it)
  - [4. Update `motoko-core`](#4-update-motoko-core)
  - [5. Update `motoko-base`](#5-update-motoko-base)
  - [Downstream](#downstream)
- [Making draft / pre-releases](#making-draft--pre-releases)
  - [Version suffix](#version-suffix)
- [Coverage report](#coverage-report)
- [Profile the compiler](#profile-the-compiler)
- [Benchmarking the RTS](#benchmarking-the-rts)

## Nix setup

The Motoko build system relies on [Nix](https://nixos.org/) to manage
dependencies, drive the build and run the test suite. You should install `nix` by
running, as a normal user with `sudo` permissions,
```
sh <(curl -L https://nixos.org/nix/install) --daemon
```

This repository is also a Nix Flake which means you need to
allow this feature by making sure the following is present in `/etc/nix/nix.conf`:
```
extra-experimental-features = nix-command flakes
```

You should also enable a nix cache to get all dependencies pre-built.

The `cachix` command also requires `sudo` permissions.
```
nix profile install --accept-flake-config nixpkgs#cachix
cachix use ic-hs-test
```
Technically, this is optional, but without this you will build lots of build
dependencies manually, which can take several hours.

## Installation using Nix

If you want just to _use_ `moc`, you can install the `moc` binary into your `nix`
environment by running
```
$ nix profile install .#release.moc
```
in a check-out of the `motoko` repository.

### Other tools

Similarly the other tools can be installed using
```
$ nix profile install .#release.mo-doc
```
etc.

## Development using Nix

To enter a shell with the necessary dependencies available,
either run:

```
$ nix develop
```

Or use `direnv` by:

* Installing: [direnv](https://direnv.net/).

* Installing: [nix-direnv](https://github.com/nix-community/nix-direnv).

* `cd` to this directory.

* `direnv allow` (only needs to be done once).

Then all tools to develop Motoko will be loaded automatically everytime you `cd`
to this directory or everytime you update `flake.{nix,lock}`.

(The first shell start may take several minutes, afterwards being much faster.)

Within this shell you can run
 * `make` in `src/` to build all binaries,
 * `make moc` in `src/` to build just the `moc` binary,
 * `make DUNE_OPTS=--watch moc` to keep rebuilding as source files are changing
 * `make` in `rts/` to build the Motoko runtime
 * `make` in `test/` to run the test suite.

This invokes `dune` under the hood, which will, as a side effect, also create
`.merlin` files for integration with Merlin, the Ocaml Language Server

## Replicating CI locally

A good way to check that everything is fine, i.e. if this will pass CI, is to run
```
$ nix build --no-link
```

For more details on our CI and CI setup, see `CI.md`.


## Making releases

We make frequent releases, at least weekly. The steps to make a release are:

Before starting the release process, ensure you are working with the latest version of the codebase. Run the following commands:

```bash
git switch master
git pull
```

Make sure the markdown doc for base is up-to-date:
For now, in a nix shell `$ nix develop` (or _re-enter_ if you already have one open):

```bash
  make -C rts
  make -C src
  make -C doc base
  git diff
```

If not, create and merge a separate PR to update the doc (adding any new files) and goto step 0.

### 1. Update Changelog

Check the recent changes from the last release:
```bash
git log --first-parent $(git describe --abbrev=0)..HEAD
```
Or, on macOS, in a browser:
```bash
open "https://github.com/dfinity/motoko/compare/$(git describe --abbrev=0)...master"
```

Look at changes and check that everything relevant is mentioned in the changelog section,
and possibly clean it up a bit, curating the information for the target audience.

You can get the latest released version with:

```bash
git describe --abbrev=0
```

Make sure that the very top of `Changelog.md` **exactly** matches the following format (otherwise the release extraction script will fail):

```markdown
# Motoko compiler changelog

## X.Y.Z (YYYY-MM-DD)

...changelog content for this version...

## ...previous version...
```

### 2. Open a release PR

Define a shell variable `NEXT_MOC_VERSION` with the next version number.
The following command extracts it from the first heading in `Changelog.md` (which you just edited).
Verify the version is correct.

```bash
echo "Last version: $(git describe --abbrev=0)"
export NEXT_MOC_VERSION=$(sed -nE 's/^## ([0-9]+\.[0-9]+\.[0-9]+) .*/\1/p' Changelog.md | head -1)
echo "Next version: $NEXT_MOC_VERSION"
```

Run the following command pipeline to create the release PR:

```bash
(test -n "$NEXT_MOC_VERSION" || (echo "NEXT_MOC_VERSION is not set" && false)) && \
git switch -c $USER/$NEXT_MOC_VERSION && \
git add Changelog.md && \
git commit -m "chore: Releasing $NEXT_MOC_VERSION" && \
git push --set-upstream origin $USER/$NEXT_MOC_VERSION && \
gh pr create --title "chore: Releasing $NEXT_MOC_VERSION" --label "release" --base master --head $USER/$NEXT_MOC_VERSION --body "" && \
gh pr merge --squash --auto
```

<details>
<summary>Or click here for detailed steps:</summary>

Switch to a new release branch (creating it if it doesn't exist):

```bash
git switch -c $USER/$NEXT_MOC_VERSION
```

Commit the changes with exactly the following message:

```bash
git add Changelog.md
git commit -m "chore: Releasing $NEXT_MOC_VERSION"
```

Push the branch:

```bash
git push --set-upstream origin $USER/$NEXT_MOC_VERSION
```

Create a PR from this commit:
- Make sure the **PR title** is the same as the **commit message**.
- Label the PR with `release` (to mark it as a release PR) and enable auto-merge on it. It will get merged into `master` without additional approval, and it may take some time as the title (version number) enters into the `nix` dependency tracking.

To create the PR, you can use `gh` CLI:
```bash
gh pr create --title "chore: Releasing $NEXT_MOC_VERSION" --label "release" --base master --head $USER/$NEXT_MOC_VERSION --body "" && gh pr merge --squash --auto
```
</details>

The PR will be merged automatically once the CI passes.
You can check the status of the PR on GitHub with
```bash
gh pr view --web
```

After the PR is merged, the `release-pr.yml` workflow should automatically create a tag and push it to the remote repository starting the release process.

### 3. Wait for the release to complete, and verify it

Verify that the release is complete and go to the next step if the release was successful.
Otherwise, fix the issue and push the tags manually as described below:

<details>
<summary>Click here for manual tag-pushing steps if the automated release fails.</summary>

After the PR is merged: Pull the latest `master` and verify you are at the right commit:

```bash
git switch master; git pull --rebase
git show
```

Push the tag:

```bash
git tag $NEXT_MOC_VERSION -m "Motoko $NEXT_MOC_VERSION"
git push origin $NEXT_MOC_VERSION
```

Pushing the tag should cause GitHub Actions to create a "Release" on the GitHub
project. This will fail if the changelog is not in order (in this case, fix and
force-push the tag).  It will also fail if the nix cache did not yet contain
the build artifacts for this revision. In this case, restart the GitHub Action
on GitHub's UI.
</details>

### 4. Update `motoko-core`

From the `main` branch, push a tag for the new `moc` version:

```bash
git checkout main
git pull
git tag moc-$NEXT_MOC_VERSION
git push origin moc-$NEXT_MOC_VERSION
```

### 5. Update `motoko-base`

From the `master` branch, push a tag for the new `moc` version:

```bash
git checkout master
git pull
git tag moc-$NEXT_MOC_VERSION
git push origin moc-$NEXT_MOC_VERSION
```

<details>
<summary>Click here for legacy `motoko-base` update steps.</summary>

After releasing the compiler, update `motoko-base`'s `master` branch to the `next-moc` branch.

* Wait ca. 5min after releasing to give the CI/CD pipeline time to upload the release artifacts
* Change into `motoko-base` and pull the latest `next-moc`
```bash
git switch next-moc; git pull
```
* Revise and update the `CHANGELOG.md`, by adding a top entry for the release

* Bump `moc` and create a PR:
```bash
# Create a new branch for the update
git switch -c $USER/update-moc-$NEXT_MOC_VERSION && \

# Update the `moc_version` env variable in `.github/workflows/{ci, package-set}.yml` and `mops.toml` to the new released version
perl -pi -e "s/moc_version: \"\\d+\.\\d+\.\\d+\"/moc_version: \"$NEXT_MOC_VERSION\"/g" .github/workflows/ci.yml .github/workflows/package-set.yml && \
perl -pi -e "s/moc = \"\\d+\.\\d+\.\\d+\"/moc = \"$NEXT_MOC_VERSION\"/g; s/version = \"\\d+\.\\d+\.\\d+\"/version = \"$NEXT_MOC_VERSION\"/g" mops.toml && \

# Add the changed files and commit the changes
git add .github/ CHANGELOG.md mops.toml && git commit -m "Motoko $NEXT_MOC_VERSION" && \

# Push the branch
git push --set-upstream origin $USER/update-moc-$NEXT_MOC_VERSION && \

# Create a PR targeting `master`
gh pr create --title "Motoko $NEXT_MOC_VERSION" --base master --head $USER/update-moc-$NEXT_MOC_VERSION --body ""

```
* You can check the status of the PR on GitHub with
```bash
gh pr view --web
```
* Once CI passes, merge the PR using the _normal merge_ (not squash merge).
  > **Note:** To allow merge commits, go to the repository settings and enable merge commits. Remember to **disable it after the merge**. Unfortunately, `gh` CLI cannot update this setting without admin permissions.

It will eventually be imported into this repo by a scheduled `niv-updater-action`.

Finally tag the base release (so the documentation interpreter can do the right thing):
First, switch to `master`, pull the latest changes and verify we are at the right commit:
```bash
git switch master && git pull
git show
```
* Tag and push the release
```bash
git tag moc-$NEXT_MOC_VERSION
git push origin moc-$NEXT_MOC_VERSION
```

</details>

### Downstream

There are a few dependent actions to follow-up the release, e.g.
- `motoko` NPM package
- `vessel` package set
- `vscode` plugin
- ICP Ninja

These are generally triggered by mentioning the release in Slack.

Announcing the release towards SDK happens by triggering this GitHub action:
https://github.com/dfinity/sdk/actions/workflows/update-motoko.yml
Press the "Run workflow" button, filling in
- Motoko version: `latest`
- Open PR against this sdk branch: `master`

and then hitting the green button. This will create a PR with all necessary hash changes against that branch. There is no
need to do this immediately, you can leave the release soaking a few days. Use your own jugdement w.r.t. risk, urgency etc.

If you want to update the portal documentation, typically to keep in sync with a `dfx` release, follow the instructions in https://github.com/dfinity/portal/blob/master/MAINTENANCE.md.

## Making draft / pre-releases

To make a draft / pre-release, you can use the GitHub Actions workflow:
https://github.com/dfinity/motoko/actions/workflows/release.yml

1. Press the **"Run workflow"** button
2. Select the branch for which you want to make the draft / pre-release
3. Fill in the **"Version suffix"** that will be used to contruct the version name, e.g. `alpha-1` version suffix could produce a version like `0.16.3-alpha-1`
4. Hit the green button to run the workflow and wait for it to complete.
5. View the draft release at https://github.com/dfinity/motoko/releases once the workflow is complete.
6. To make a pre-release:
   1. Edit the draft release
   2. Make sure the tag and the name of the release are the same as the generated version, e.g. `0.16.3-alpha-1`. Note that there is no need to manually push the tag, it should be created automatically when publishing the pre-release.
   3. Scroll down to the bottom and publish the pre-release.
7. Remember to delete the draft release after testing!

### Version suffix

The version is generated by concatenating the 'latest `moc` version' with the version suffix, e.g. `0.16.3-alpha-1` for the version suffix `alpha-1`.
The 'latest `moc` version' is taken from the first matching entry in `Changelog.md` **on the selected branch**. Source code: [nix/releaseVersion.nix](nix/releaseVersion.nix).

The workflow should upload the artifacts with correct names according to the generated version.

The generated version (e.g. `0.16.3-alpha-1`) should be used as the name of the release and the tag.
Artifacts should be uploaded with the correct names according to the generated version, e.g. `motoko-Darwin-arm64-0.16.3-alpha-1.tar.gz` -- This is necessary for `mops toolchain` to fetch the correct file.

## Coverage report

To build with coverage enabled, compile the binaries in `src/` with
```
make DUNE_OPTS="--instrument-with bisect_ppx"`
```
and then use `bisect-ppx-report html` to produce a report.

The full report can be built with
```
nix build .#tests.coverage
```
and the report for latest `master` can be viewed at
[https://dfinity.github.io/motoko/coverage/](https://dfinity.github.io/motoko/coverage/).

## Profile the compiler

(This section is currently defunct, and needs to be update to work with the dune
build system.)

1. Build with profiling within nix-shell (TODO: How to do with dune)
   ```
   make -C src clean
   make BUILD=p.native -C src moc
   ```
2. Run `moc` as normal, e.g.
   ```
   moc -g -c foo.mo -o foo.wasm
   ```
   this should dump a `gmon.out` file in the current directory.
3. Create the report, e.g. using
   ```
   gprof --graph src/moc
   ```
   (Note that you have to _run_ this in the directory with `gmon.out`, but
   _pass_ it the path to the binary.)


## Benchmarking the RTS

Specifically some advanced techniques to obtain performance deltas for the
GC can be found in `rts/Benchmarking.md`.
