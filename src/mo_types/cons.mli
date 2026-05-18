(* Generative constructors *)

type scope = string

type 'a t

(* Reset to the original state after running the provided function. The [scope]
   is used to create a new unique stamp for each constructor per file. *)
val session : ?scope:scope -> (unit -> 'a) -> 'a

val fresh : string -> 'a -> 'a t
val clone: 'a t -> 'a -> 'a t

(** Mint a "skolem" cons — stamp drawn from the [<= 0] reserve.
    [is_skolem] returns [true] for the result.  Today the only
    caller is the GADT existential machinery; future deterministic
    stamping (e.g. [-Hashtbl.hash (region, schema_stamp)]) will live
    in the same range without changing [is_skolem]'s contract. *)
val fresh_skolem : string -> 'a -> 'a t

(** [true] iff [c]'s stamp is in the [<= 0] reserve (i.e., minted via
    [fresh_skolem] or — eventually — a deterministic stamping helper). *)
val is_skolem : 'a t -> bool

val name : 'a t -> string

val to_string : bool -> string -> 'a t -> string

val kind : 'a t -> 'a
val unsafe_set_kind : 'a t -> 'a -> unit (* cf. Type.set_kind *)

val eq : 'a t -> 'a t -> bool
val compare : 'a t -> 'a t -> int
