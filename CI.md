Motoko CI and CD setup
======================

This file gives a comprehensive overview of our CI and CD setup. Target
audience are developers working on Motoko (to know what services and behaviours
they can expect), and engineers who support the setup.

This document distinguishes use-cases (requirements, goals) from
implementations, to allow evaluating alternative implementations.

The implementation is currently a careful choreography between Github, Github
Actions, and the Cachix nix cache.

Knowing if it is broken
-----------------------

**Use-case:**
Everything is built and tested upon every push to a branch of the repository,
and the resulting status is visible (at derivation granularity) to developers.

**Implementation:**
All pushes to any branch are built by a Github Action job, on Linux and Darwin.

The build status is visible via the Github status (coarsely: only evaluation,
`{debug,release}-systems-go`).

This includes linux and darwin builds.

CI runners and remote builds
----------------------------

**Use-case:**
Builds and tests should run on fast, cost-effective infrastructure. The big
expensive GitHub runners (`ubuntu-24-large`, `arm64-linux-16`) only have 16
shared CPUs, so all derivations of a job compete for them.

**Implementation:**
Linux `nix` builds are offloaded to [nixbuild.net](https://nixbuild.net), a
remote Nix builder, via the [`nixbuild/nixbuild-action`](https://github.com/marketplace/actions/nixbuild-net)
and [remote store builds](https://docs.nixbuild.net/remote-builds/#using-remote-stores).
Each derivation gets its own machine (up to 16 CPUs) instead of sharing one
runner, which is both faster and cheaper. Because the GitHub runner then does
almost no work, the Linux jobs run on the small/free runners (`ubuntu-latest`,
`ubuntu-24.04-arm`) instead of `ubuntu-24-large` / `arm64-linux-16`.

This mirrors the earlier setup from
[PR #5032](https://github.com/caffeinelabs/motoko/pull/5032) (later reverted in
[#5694](https://github.com/caffeinelabs/motoko/pull/5694)).

Mechanics:
 * `nixbuild.sh` runs `nix build` against the `ssh-ng://eu.nixbuild.net` store on
   Linux (and locally elsewhere). `nixbuildcopy.sh` additionally copies the
   result back to the runner for steps that need the artifacts locally
   (releases, benchmark outputs, the user guide).
 * The `test-blueprint` action sets up nixbuild.net for Linux jobs that have a
   `nixbuild_token`. macOS has no nixbuild.net support, so it builds locally.
 * Fork PRs cannot read repository secrets, so `nixbuild_token` is empty for
   them; the Linux path then falls back to a local `nix-build-uncached` build so
   fork CI keeps working (without the remote-build speedup). The heavy
   `gc-tests` / `tests` jobs pass `max-jobs: 1` for this fallback so the ~3 GB
   RTS-variant builds run one at a time and don't OOM the small standard runner
   (this input is ignored on the nixbuild.net path).
 * Authentication uses the `NIXBUILD_TOKEN` repository secret.

macOS (nightly tests, `build`/`release` macOS targets) stays on GitHub-hosted
runners. nixbuild.net has no macOS support; if GitHub macOS minutes become a
cost concern, [Namespace](https://namespace.so) offers macOS runners.

**Aborting superseded runs:**
`test.yml` uses a `concurrency` group keyed by PR number with
`cancel-in-progress` enabled for pull requests, so rapid successive pushes to a
PR cancel the outdated in-progress run. Pushes to `master` and merge-queue runs
are never cancelled.

Preventing `master` from breaking
---------------------------------

**Use-case:**
A PR that breaks requires jobs (`all-systems-go`) cannot be merged into `master`.

**Implementation (external):**
Github branch protection is enabled for `master`, and requires the
Github Action jobs (Linux and Darwin) to succeed.

Require a second pair of eyeballs
---------------------------------

**Use-case:**
A PR needs to be approved by any other developer in order to be merged (with
the exceptions listed below).

**Implementation:**
Github branch protection requires a review.

Warm cache
----------

**Use-case:**
Developers can rely on all build artifacts (espcially, but not exclusively, the
dependencies of the nix shell) being present in a nix cache.

**Implementation (external):**
Github Actions pushes all builds to the public cachix.org-based nix cache.

**Implementation (internal):**
Hydra pushes all builds to the internal nix cache.

Push releases
-------------

**Use-case:**
Tagged versions cause a tarball with a Motoko release to be pushed to
https://github.com/dfinity/motoko/releases

**Implementation (external):**
A github action creates Github releases and includes the build artifacts there.

Automatically merge when ready
------------------------------

**Use-case:**
Developers can indicate that a PR should be merged as soon as all requirements
pass, using the PR description as the commit message.

This can be done before approvals are in and/or CI has turned green, and will
reliably be acted on once requirements are fulfilled.

**Implementation:**
Use the "Enable auto-merge (squash)" button of the GitHub PR web page.
Alternatively from the CLI `gh pr merge --squash --auto` will do. Note that you have to update
the PR with `master` first, so that the tests can run on the most recent codebase.
From the command line issue `gh pr update-branch` to that effect.

Render and provide various reports
----------------------------------
**Use-case:**
Various build artifacts are most useful when available directly in the browser, namely:

 * The motoko user guide
 * The “overview slides”
 * The documentation for `motoko-base`, in the version pinned by motoko
 * Flamegraphs for the programs in `tests/perf`
 * A coverage report

A stable link to these should exist for `master`, and an easy-to-find link for each PR.

**Implementation (internal):**
Hydra hosts the build products of the relevant jobs, and can be found via the
Hydra job status page, and the there is a stable link for the latest build of
`master` and of each PR.

**Implementation (external):**
The latest `master` version of the file is availble at
[https://dfinity.github.io/motoko/](https://dfinity.github.io/motoko/).
The reports are calculated in PRs (so failures would be caught), but are not
hosted anywhere.

Performance changes are known
-----------------------------

**Use-case:**
For every PR, the developer is told about performance changes relative to the
merge point, via an continuously updated comment on the PR.

**Implementation (external):**
 * Steps in the Github Action calculates the correct merge base using
   `git-merge-base` (_not_ the latest version of the target branch) and passes
   the correct git revisions to the `./perf-delta.nix` nix derivation.
 * Building that derivations compares metrics and generates a report.
 * A Github Action updates the comment upon every new push to the PR.

**Implementation (internal):**
 * Hydra calculates the correct merge base using `git-merge-base` (_not_ the
   latest version of the target branch) and passes a checkout of that revision
   as `src.mergeBase` to `ci-pr.nix`.
 * The job `perf-delta` compares metrics and generates a report.
 * The hydra Github commenter plugin updates the comment upon every new push to
   the PR.

Dependencies are updated automatically
--------------------------------------

**Use-case:**
Several dependencies, as pinned by `flake.nix`, should be updated without
human intervention. Some dependencies are updated daily, others weekly. For
some dependency, it should only be _tested_ if it builds, but not merged.

**Implementation:**
 * Multiple files (with different settings) in `.github/workflows/` use
   flake-updater to create pull requests with the version bumps,
   as `github-actions[bot]`, setting `automerge-squash` or `autoclose`.
 * A GH action automatically approves PRs from `github-actions[bot]`.
 * Once CI passes, the `test.yml` GitHub action merges or closes PRs, as per label.

(obsolete) Updates to the Changelog require no review
------------------------------------------

**Use-case:**
To make releasing releases frictionless (see section in `Building.md`), PRs
that only update `Changelog.md` do not require a human approver.

**Implementation:**
Mergify approves PRs that only change the `Changelog.md` file.
