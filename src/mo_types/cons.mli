(* Generative constructors *)

type scope = string

type 'a t

(* Reset to the original state after running the provided function. The [scope]
   is used to create a new unique stamp for each constructor per file. *)
val session : ?scope:scope -> (unit -> 'a) -> 'a

val fresh : string -> 'a -> 'a t
val clone: 'a t -> 'a -> 'a t

val name : 'a t -> string

val to_string : bool -> string -> 'a t -> string

val bump_stamps_past : 'a t list -> unit

val kind : 'a t -> 'a
val unsafe_set_kind : 'a t -> 'a -> unit (* cf. Type.set_kind *)

val eq : 'a t -> 'a t -> bool
val compare : 'a t -> 'a t -> int
