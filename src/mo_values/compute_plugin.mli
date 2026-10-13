(* Synchronous compute plugins. The native driver installs trusted compiler
   extensions before loading Prim. Results are ordinary managed Motoko values.
 *)
open Mo_types

(** Nominal, non-shared and non-stable plugin values backed by immutable
    managed Blobs. Payloads must contain only initialized pointer-free bytes;
    no native pointers or native layouts with uninitialized padding. Payload
    alignment is eight bytes. A plugin owns and validates its byte format. *)
module Prepared : sig
  type t
  val create : owner:string -> name:string -> t
  val typ : t -> Type.typ

  (** Reference evaluators use the same Blob representation as compiled code.
      These helpers require a registered descriptor. They do not copy bytes or
      validate the plugin's already prepared payload again. *)
  val wrap : t -> string -> Value.value
  val unwrap : t -> Value.value -> string

  (** Compiler integration: register once, then recognize exact constructor
      identity. Alias normalization is allowed; abstract bounds are not. *)
  val register : owner:string -> t -> unit
  val check_owner : owner:string -> t -> unit
  val is_con : Type.con -> bool
  val find : string -> Type.typ option
  val check_type : t -> Type.typ -> unit
end

(* All bindings are compiler-specialized intrinsics, including through aliases.
   [Shared] accepts variadic shared values; [Fixed] declares input types. *)
type type_parameter = Concrete | Opaque

type arguments =
  | Shared
  | Fixed of (Type.typ list -> (Type.typ list, string) result)

(* Calls are ordinary synchronous local Motoko calls. The continuation retains
   the interpreter's scheduling and actor context; plugins never extract a
   closure and assume that invoking it completes synchronously. *)
type evaluator = {
  call : Type.typ -> Value.value -> Value.value list ->
    ((Value.value, string) result -> unit) -> unit;
}

type specialization = {
  type_arguments : Type.typ list;
  argument_types : Type.typ list;
  result_type : Type.typ;
}

type binding = {
  id : string;
  name : string;
  arguments : arguments;
  type_parameters : type_parameter list;
  result_type : Type.typ list -> Type.typ list -> (Type.typ, string) result;
  evaluate : evaluator -> Type.typ list -> Type.typ list ->
    Value.value list -> ((Value.value, string) result -> unit) -> unit;
}

(* Lift a callback-free reference computation into the evaluator protocol. *)
val pure : (Type.typ list -> Type.typ list -> Value.value list ->
  (Value.value, string) result) -> evaluator -> Type.typ list ->
  Type.typ list -> Value.value list -> ((Value.value, string) result -> unit) -> unit

val register : binding -> unit
val find : string -> binding option
val declarations : unit -> string
val intrinsic_type : string -> Type.typ
val intrinsic : Type.typ -> string option
val specialize : binding -> Type.typ list -> Type.typ list ->
  (specialization, string) result
val map_specialization : (Type.typ -> Type.typ) -> specialization ->
  specialization
(* Execute metadata checked by the frontend or IR checker. Dynamic argument
   arity and callback calling conventions are still checked during evaluation. *)
val evaluate : Value.context -> string -> specialization ->
  Value.value list -> ((Value.value, string) result -> unit) -> unit
