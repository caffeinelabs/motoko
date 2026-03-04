(* Numeric Representations *)

module type NumType =
sig
  type t
  val signed : bool
  val zero : t
  val one : t
  val abs : t -> t
  val neg : t -> t
  val add : t -> t -> t
  val sub : t -> t -> t
  val mul : t -> t -> t
  val div : t -> t -> t
  val rem : t -> t -> t
  val pow : t -> t -> t
  val eq : t -> t -> bool
  val ne : t -> t -> bool
  val lt : t -> t -> bool
  val gt : t -> t -> bool
  val le : t -> t -> bool
  val ge : t -> t -> bool
  val compare : t -> t -> int
  val to_int : t -> int
  val of_int : int -> t
  val to_int32 : t -> Int32.t
  val of_int32 : Int32.t -> t
  val to_int64 : t -> Int64.t
  val of_int64 : Int64.t -> t
  val to_big_int : t -> Big_int.big_int
  val of_big_int : Big_int.big_int -> t
  val of_string : string -> t
  val to_string : t -> string
  val to_pretty_string : t -> string
end

(* Extension of NumType with wrapping and bit-wise operations *)
module type BitNumType =
sig
  include NumType

  val not : t -> t
  val popcnt : t -> t
  val clz : t -> t
  val ctz : t -> t

  val and_ : t -> t -> t
  val or_ : t -> t -> t
  val xor : t -> t -> t
  val shl : t -> t -> t
  val shr : t -> t -> t
  val rotl : t -> t -> t
  val rotr : t -> t -> t

  val wrapping_of_big_int : Big_int.big_int -> t

  val wadd : t -> t -> t
  val wsub : t -> t -> t
  val wmul : t -> t -> t
  val wpow : t -> t -> t
end

module type FloatType =
sig
  include Wasm.Float.S
  val rem : t -> t -> t
  val pow : t -> t -> t
  val to_pretty_string : t -> string
end

module Float : FloatType with type bits = int64 and type t = Wasm.F64.t

module Int : NumType
module Nat : NumType with type t = Int.t
module Int_8 : BitNumType
module Int_16 : BitNumType
module Int_32 : BitNumType
module Int_64 : BitNumType
module Nat8 : BitNumType
module Nat16 : BitNumType
module Nat32 : BitNumType
module Nat64 : BitNumType

val bigint_of_double : Wasm.F64.t -> Big_int.big_int
