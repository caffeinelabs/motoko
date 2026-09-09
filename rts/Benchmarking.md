# How I benchmark GC changes

This documents how I (Ömer) benchmark changes to the garbage collectors.

In principle it shouldn't be too difficult to benchmark GC in isolation, as we
can already run GC in runtime system test suite (see
`rts/motoko-rts-tests/src/gc.rs`).

However, there are a few challenges with this:

- RTS tests need to be run with a Wasm interpreter/runtime that supports WASI.
  Currently we use wasmtime, and I'm not sure if it's possible to generate Wasm
  instructions executed, or used/dirtied Wasm pages etc. with wasmtime.

  If there are other Wasm runtimes (with WASI support) that can generate these
  information, then those could be used to run GC benchmarks implemented in
  Rust, similar to the GC tests.

- It may be tricky to generate large and realistic Motoko heaps with in Rust.

So currently I'm using a Motoko canister to benchmark GC changes.

## Helpful flags

moc flags that are helpful when benchmarking GCs:

- `--force-gc`: This flag disables GC scheduling and forces GC after every
  update call.

  **Why is this needed?** With scheduling the canister needs to allocate large
  amounts to do GC. When benchmarking GC, ideally I want to do GC 100% of the
  time and nothing else. With this flag, we still run the mutator code, but GC
  runs more often, so the noise from mutator executions becomes less.

  This flag should always be used when micro-benchmarking GC.

## Building the hacky replica to generate canister stats

You should be using drun from `roman/hypervisor-instr-hack` branch of DFINITY.

This version of drun generates a `canister_perf.csv` file after each run with
these columns:

```
canister,method,instructions,accessed_pages,dirtied_pages,total_pages_in_use
```

Notes:

- `accessed_pages` and `dirtied_pages` are in **host** pages, not in Wasm
  pages. A host page is commonly 4 KiB on Linux. When in doubt, run `getconf
  PAGE_SIZE` to get the page size.

  **Reminder:** A Wasm page is 64 KiB.

  **Note:** This means replica has more fine-grained page tracking than Wasm.
  For example, if a canister modifies first 4 KiB of a Wasm page, replica
  doesn't dirty 16 pages, just the OS page the canister modified.

- `total_pages_in_use` is **Wasm** pages, not host pages!

drun can be built with the latest stable Rust toolchain (1.56 as I write this)
by just navigating into `rs/drun` and running `cargo build --release`. No need
for nix. The executable will be in `rs/target/release/drun`.

Copy this executable to your `$PATH` for ease of access.

## Finding a canister

So far the only canister I have that is suitable for benchmarks is
https://github.com/dfinity/cancan-archived. I'm using commit 302343a.

Run `vessel sources` to get `--package` flags for the dependencies:

```
$ vessel sources
--package base .vessel/base/dfx-0.7.0-beta.2/src --package crud .vessel/crud/master/src --package sequence .vessel/sequence/master/src
```

Note that the base version above is buggy and you need the master branch, so
clone motoko-base and update the path above with the path to your clone.


```
<path to moc> <package flags> service/CanCan.mo -o <wasm file name> --force-gc <gc strategy>
```

- `<path to moc>`: Path to `moc` executable
- `<package flags>`: Flags printed by the vessel command shown above. Make sure
  to update base path to point at motoko-base master.
- `<wasm file name>`: Name of the generated .wasm file. Make sure to add some
  prefix/suffixes to this binary to describe the compile flags. E.g.
  `cancan_incremental_gc_my_awesome_patch_tweaked_scheduling_wow.wasm`.

(The classical `--copying-gc` and `--compacting-gc` GC strategies were removed
in the 1.16 → v2 migration; `moc` always uses the incremental GC. Use
`--force-gc` for GC benchmarking as described above.)

## Writing a drun script

Now that we have a canister, we need a drun script to install the canister, and
send messages to it. The resulting `canister_perf.csv` file will have one line
for each `ingress` line in the drun script.

For the CanCan backend canister above, I use a script like this:

```
create
install rwlgt-iiaaa-aaaaa-aaaaa-cai <canister Wasm path> ""
ingress rwlgt-iiaaa-aaaaa-aaaaa-cai createProfile "DIDL\x02n\x01m{\x02q\x00\x05test0\x00"
ingress rwlgt-iiaaa-aaaaa-aaaaa-cai createProfile "DIDL\x02n\x01m{\x02q\x00\x05test1\x00"
ingress rwlgt-iiaaa-aaaaa-aaaaa-cai createProfile "DIDL\x02n\x01m{\x02q\x00\x05test2\x00"
...
```

This calls `createProfile` a few hundred times.

Now I save this as `test.drun` run drun:

```
drun test.drun
```

After that I have `canister_perf.csv` file with the results. Make sure to copy
this CSV file because if you accidentally run drun again (with up-arrow + enter
or something like that) it will destroy your results.

## Comparing results

- Compile the canister with different flags, with different output file names.
- Run the drun script twice, updating the canister Wasm path in the drun script
  before running drun.
- Copy generated .csv file after each run, give it a descriptive name.

After that you will have csv files that you can compare.

## How to compare CSV files

Most of the time I simply do `nvim -d <csv1> <csv2>`.

I have a [Rust program](https://github.com/osa1/generate_drun_bench_plots) to generates plots for instructions, dirtied pages etc.
for CSV files.
