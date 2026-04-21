(** Compiler phase timings for benchmarking ([--emit-compiler-timings]). *)

val clear : unit -> unit

val with_phase : string -> string -> (unit -> 'a) -> 'a
(** [with_phase heading name thunk] runs [thunk] and records wall-clock duration
    when [--emit-compiler-timings] is set. *)

val maybe_write : unit -> unit
(** If [--emit-compiler-timings] was given, write JSON to that path.
    Call only after a successful compilation run (no partial timings).
    The JSON includes [phases] (per unit) and [phase_totals] (sum of [duration_ns] per phase name). *)
