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
The latest `master` version of the file is available at
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
