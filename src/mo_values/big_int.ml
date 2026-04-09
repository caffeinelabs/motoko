(* Module alias: re-exports the Bigint virtual library as Big_int.
   The compiler source files (numerics.ml, prim.ml) reference Big_int,
   which originally came from the `num` package. We replaced the `num`
   dependency with a virtual library `bigint_api` so that native builds
   use the real `num` (via bigint_num) while wasm_of_ocaml builds use
   a lightweight Int64-backed shim (via bigint_lite) that avoids C stubs.
   This alias lets all existing source code keep using Big_int unchanged. *)
include Bigint
