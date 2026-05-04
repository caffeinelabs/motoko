The Motoko test suite
==========================

Commands
--------

Run these either in the top level directory, or in one of the subdirectories.

* `make`

   Runs all tests, fails if any fail.

* `make accept`

   Refreshes all output

* `make clean`

   Cleans

You can also run individual tests, for example:

    run-test run/fac.mo

to run and

    run-test -a run/fac.mo

to accept. Check `run-test --help` for other flags (e.g. drun-mode), and see
`Makefile` in each subdirectory for the right flags for that directory.

Adding a new test
-----------------

1. Create `foo.mo` (or similar, see below)
2. Run `make accept` (or, more targeted, `run-test -a foo.mo`)
3. Add `foo.mo` and `ok/foo.*.ok` to git.

`run-test` takes various flags, e.g. `-d` to compile actors instead of program,
`-p` for performance measurements. Each subdirectory has a `Makefile` specifying
these flags.

Kinds of tests
--------------

`run-test` supports different kinds of tests:

### Single Motoko file tests

These consist of a single Motoko file, e.g. `foo.mo`, which will be
typechecked, interpreted (in various variants) and run on `wasmtime` or (with
`-d`) `drun`.

With comments of the form `//SKIP run-low` individual phases can be skipped.
Similarly, mentioning the `uname` output (like `//SKIP Darwin`) skips the test
when running on that OS.

Comments of the form `//MOC-FLAG --package prim .` pass additional flags to
`moc`.

Comments of the form `//CALL` will be picked up and passed to `drun` as additional calls to be made.  The variant `//OR-CALL` will
remove that line. This allows different behavior with the interpreter and in
`drun`. See existing files for details.

### Multiple Motoko test files

These only make sense with `-d`. Create a `foo.drun` file that is a mostly
unmodified input with one exception: You can reference `foo/bar.mo` files where
`drun` expects a `.wasm` file. `run-test` will find these files, compile them
to `.wasm` and put that file name into the script before passing it to `drun`.

### Shell files

Files named `foo.sh` will simply be executed.

### `.wat` files

Files named `foo.wat` expect a corresponding `.c` file and are used to test
`mo-ld`. See `ld/` for examples.

### `.did` files

Files named `foo.did` will be passed through the `didc` file checker,
pretty-printer, the pretty-printed file will be checked again. It also generates
JS bindings, which will be parsed by `node`.


Running as a `nix` derivation
-----------------------------

You can run the test suite from the `motoko` (top-level) directory as:

``` shell
$ nix-build -A tests
```

You can also run individual directories via, say,

``` shell
$ nix-build -A tests.run-drun
```

Running WASI tests in the browser
---------------------------------

The browsers provide a developer console that supports some support for
stepping through wasm (including pretty-printing WASM, breakpoints, stepping).
Together with the ability to print, this can be useful for debugging.

You can easily run any of the tests in `test/run` in the browser as follows:

* Make sure they are built:
  ```
  make -C run
  ```
  (or just `run-test run/empty.mo` to just build a single one.)

* Run the python web server:
  ```
  python3 -m http.server
  ```
  (It likely has to be this one, as the script parses the directory listing)

* Open the URL that this command tells you, likely http://0.0.0.0:8000/

Now you can select the test you are interested from the drop down. It will load the wasm and run it. You can open the debugger, insepct the wasm, set breakpoints.

Use the _Reload_ button if you have changed the `.wasm.`;
use the _Rerun_ button if you want to rerun from the beginning without reloading, e.g. after setting break points.

Randomised testing
------------------

See `README.md` in the `random/` subdirectory.

Performance regression testing
------------------------------

The purpose of the `perf/` directory is to have a small (\<20) set of test
programs representative of real use of Motoko.

For these tests the test suite records the following numbers:

* Size of the produced Wasm binary.
* Cycles consumed by a single run in drun

The numbers are written to the file specified by `$PERF_OUT` (and end up being
the output of the nix derivation `tests.perf`).

The format is a simple CSV format, as consumed by
[gipeda](https://github.com/nomeata/gipeda).

Every PR reports a summary of changes to these numbers to the PR.

Wasm profiling/flamegraphs
--------------------------

The programs in the `perf/` directory can also be used to get some
instruction-based profiling data/reports, using the
[wasm-profiler](https://github.com/dfinity/wasm-profiler).

To generate the report, run

    ./profile-report.sh

and look in `_profile/`.

The same can be achieved with

    nix-build -A tests.profiling-graphs ..

and Hydra serves [the latest report].

[the latest report]: https://hydra.dfinity.systems/job/dfinity-ci-build/motoko/tests.profiling-graphs/latest/download/1/index.html

Also see this script for inspiration if you want to profile other programs or
do other things.

Candid test suite
-----------------

To run the candid test suite, just run the

    candid-tests

command.

To run it against a local copy of the test data, pass `-i ../candid/tests/`.

To mark certain tests as known-to-be-failing, pass `--expect-fail` in the
invocation to `candid-tests` in `default.nix`.

To view the generated Motoko code for the tests, pass `--diag`.

See `candid-tests --help` for instructions.
