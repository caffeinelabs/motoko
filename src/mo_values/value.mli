open Mo_types
open Numerics

module Blob : sig
  val escape : string -> string
  val rand32 : unit -> string
end

(* Environment *)

module Env : Env.S with type key = string


(* Types *)

type unicode = int
type actor_id = string

type context = value

and func =
  context -> value -> value cont -> unit

and comp =
  value cont -> value cont -> unit

and weak =
  value Weak.t

and value =
  | Null
  | Bool of bool
  | Int of Int.t
  | Int8 of Int_8.t
  | Int16 of Int_16.t
  | Int32 of Int_32.t
  | Int64 of Int_64.t
  | Nat8 of Nat8.t
  | Nat16 of Nat16.t
  | Nat32 of Nat32.t
  | Nat64 of Nat64.t
  | Float of Float.t
  | Char of unicode
  | Text of string
  | Blob of string
  | Tup of value list
  | Opt of value
  | Variant of string * value
  | Array of value array
  | Obj of value Env.t
  | Func of Call_conv.t * func
  | Async of async
  | Comp of comp
  | Mut of value ref
  | Iter of value Seq.t ref (* internal to {b.values(), t.chars()} iterator *)
  | Weak of weak

and res = Ok of value | Error of value
and async = {result : res Lib.Promise.t ; mutable waiters : (value cont * value cont) list}

and def = value Lib.Promise.t
and 'a cont = 'a -> unit


(* Shorthands *)

val unit : value

val local_func : int -> int -> func -> value
val message_func : Type.shared_sort -> int -> func -> value
val async_func : Type.shared_sort -> int -> int -> func -> value
val replies_func : Type.shared_sort -> int -> int -> func -> value


(* Pseudo actor ids *)

val fresh_id : unit -> actor_id
val top_id : actor_id


(* Projections *)

val as_null : value -> unit
val as_bool : value -> bool
val as_int : value -> Int.t
val as_int8 : value -> Int_8.t
val as_int16 : value -> Int_16.t
val as_int32 : value -> Int_32.t
val as_int64 : value -> Int_64.t
val as_nat8 : value -> Nat8.t
val as_nat16 : value -> Nat16.t
val as_nat32 : value -> Nat32.t
val as_nat64 : value -> Nat64.t
val as_float : value -> Float.t
val as_char : value -> unicode
val as_text : value -> string
val as_blob : value -> string
val as_iter : value -> value Seq.t ref
val as_array : value -> value array
val as_tup : value -> value list
val as_unit : value -> unit
val as_pair : value -> value * value
val as_opt : value -> value
val as_obj : value -> value Env.t
val as_variant : value -> string * value
val as_func : value -> Call_conv.t * func
val as_async : value -> async
val as_comp : value -> comp
val as_mut : value -> value ref
val as_weak : value -> weak


(* Ordering *)

val equal : value -> value -> bool
val compare : value -> value -> int


(* Pretty Printing *)

(* NB: Pass Type.Non to print value at full dynamic, not static, type. *)
val pp_val : int -> Format.formatter -> (Type.typ * value) -> unit
val pp_def : int -> Format.formatter -> (Type.typ * def) -> unit

(* NB: Pass Type.Non to print value at full dynamic, not static, type. *)
val string_of_val : int -> Type.typ -> value -> string
val string_of_def : int -> Type.typ -> def -> string
