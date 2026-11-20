(* Generative constructors *)

type scope = string

type 'a t

(* Runs [f], then resets all stamp counters to their state before the call. *)
val session : (unit -> 'a) -> 'a

(* Runs [f] with stamps drawn from the counters of file [scope]. The counters
   outlive the call, so a later call with the same [scope], such as compiling a
   file after type-checking it, never hands out a stamp a constructor already has. *)
val with_scope : scope -> (unit -> 'a) -> 'a

val fresh : string -> 'a -> 'a t
val clone: 'a t -> 'a -> 'a t

val name : 'a t -> string

val to_string : bool -> string -> 'a t -> string

val kind : 'a t -> 'a
val unsafe_set_kind : 'a t -> 'a -> unit (* cf. Type.set_kind *)

val eq : 'a t -> 'a t -> bool
val compare : 'a t -> 'a t -> int
