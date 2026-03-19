(*
This module contains all the numeric stuff, culminating
in the NatN and Int_N modules, to be used by value.ml
*)

let rec add_digits buf s i j k =
  if i < j then begin
    if k = 0 then Buffer.add_char buf '_';
    Buffer.add_char buf s.[i];
    add_digits buf s (i + 1) j ((k + 2) mod 3)
  end

let is_digit c = '0' <= c && c <= '9'
let isnt_digit c = not (is_digit c)

let group_num s =
  let len = String.length s in
  let mant = Lib.Option.get (Lib.String.find_from_opt is_digit s 0) len in
  let point = Lib.Option.get (Lib.String.find_from_opt isnt_digit s mant) len in
  let frac = Lib.Option.get (Lib.String.find_from_opt is_digit s point) len in
  let exp = Lib.Option.get (Lib.String.find_from_opt isnt_digit s frac) len in
  let buf = Buffer.create (len*4/3) in
  Buffer.add_substring buf s 0 mant;
  add_digits buf s mant point ((point - mant) mod 3 + 3);
  Buffer.add_substring buf s point (frac - point);
  add_digits buf s frac exp 3;
  Buffer.add_substring buf s exp (len - exp);
  Buffer.contents buf

(* OCaml version of LibTomMath's mp_set_double
   Converts a Wasm f64 (represented as IEEE 754 double, same as OCaml `float`)
   to Big_int *)
let bigint_of_double (f : Wasm.F64.t) : Big_int.big_int =
  let bits = Wasm.F64.to_bits f in

  (* A bit pattern with 11 least significant bits set *)
  let bits_11 = Int64.of_int 0x7FF in

  (* Exponent part of IEEE 754 double, 11 bits *)
  let exp = Int64.(logand (shift_right_logical bits 52) bits_11) in

  (* Fraction part of IEEE 754 double, 52 bits from the float, with an implicit
     1 at the 53rd bit *)
  let frac = Int64.(logor (shift_right_logical (shift_left bits 12) 12) (shift_left one 52)) in

  if Int64.(equal exp bits_11) then
    (* Exponent is fully set: NaN or inf *)
    raise (Invalid_argument "bigint_of_double: argument is NaN or inf");

  (* Actual exponent value: subtract bias (1023), and 52 for the missing
     fraction dot in `frac`. Reminder: if fractional part is `xxx...` (binary)
     then actual fraction is `1.xxx...`, which we represent as `1xxx...` in
     `frac`. `- 52` here is to take that lost fraction point into account. *)
  let exp = Int64.(sub exp (of_int (1023 + 52))) in

  let a = Big_int.big_int_of_int64 frac in

  let a = if Int64.(compare exp zero) < 0 then
    (* Exponent < 0, shift right *)
    Big_int.(shift_right_big_int a (- (Int64.to_int exp)))
  else
    (* Exponent >= 0, shift left *)
    Big_int.(shift_left_big_int a (Int64.to_int exp))
  in

  (* Negate the number if sign bit is set (double is negative) *)
  if Int64.(shift_right_logical bits 63 = one) then
    Big_int.minus_big_int a
  else
    a

(* a mild extension over Was.Int.RepType *)
module type WordRepType =
sig
  include Wasm.Int.RepType
  val of_big_int : Big_int.big_int -> t (* wrapping *)
  val to_big_int : t -> Big_int.big_int
end

module Int64Rep : WordRepType =
struct
  include Int64
  let bitwidth = 64
  let to_hex_string = Printf.sprintf "%Lx"

  let of_big_int i =
    let open Big_int in
    let i = mod_big_int i (power_int_positive_int 2 64) in
    if lt_big_int i (power_int_positive_int 2 63)
    then int64_of_big_int i
    else int64_of_big_int (sub_big_int i (power_int_positive_int 2 64))

  let to_big_int i =
    let open Big_int in
    if i < 0L
    then add_big_int (big_int_of_int64 i) (power_int_positive_int 2 64)
    else big_int_of_int64 i
end


(* Represent n-bit words using k-bit words by shifting left/right by k-n bits *)
module SubRep (Rep : WordRepType) (Width : sig val bitwidth : int end) : WordRepType =
struct
  let _ = assert (Width.bitwidth < Rep.bitwidth)

  type t = Rep.t

  let bitwidth = Width.bitwidth
  let bitdiff = Rep.bitwidth - Width.bitwidth
  let inj r  = Rep.shift_left r bitdiff
  let proj i = Rep.shift_right_logical i bitdiff

  let zero = inj Rep.zero
  let one = inj Rep.one
  let minus_one = inj Rep.minus_one
  let max_int = inj (Rep.shift_right_logical Rep.max_int bitdiff)
  let min_int = inj (Rep.shift_right_logical Rep.min_int bitdiff)
  let neg i = inj (Rep.neg (proj i))
  let add i j = inj (Rep.add (proj i) (proj j))
  let sub i j = inj (Rep.sub (proj i) (proj j))
  let mul i j = inj (Rep.mul (proj i) (proj j))
  let div i j = inj (Rep.div (proj i) (proj j))
  let rem i j = inj (Rep.rem (proj i) (proj j))
  let logand = Rep.logand
  let logor = Rep.logor
  let lognot i = inj (Rep.lognot (proj i))
  let logxor i j = inj (Rep.logxor (proj i) (proj j))
  let shift_left i j = Rep.shift_left i j
  let shift_right i j = let res = Rep.shift_right i j in inj (proj res)
  let shift_right_logical i j = let res = Rep.shift_right_logical i j in inj (proj res)
  let of_int i = inj (Rep.of_int i)
  let to_int i = Rep.to_int (proj i)
  let to_string i = group_num (Rep.to_string (proj i))
  let to_hex_string i = group_num (Rep.to_hex_string (proj i))
  let of_big_int i = inj (Rep.of_big_int i)
  let to_big_int i = Rep.to_big_int (proj i)
end

module Int8Rep = SubRep (Int64Rep) (struct let bitwidth = 8 end)
module Int16Rep = SubRep (Int64Rep) (struct let bitwidth = 16 end)
module Int32Rep = SubRep (Int64Rep) (struct let bitwidth = 32 end)

(*
This WordType is used only internally in this module, to implement the bit-wise
or wrapping operations on NatN and IntN (see module Ranged)
*)

module type WordType =
sig
  include Wasm.Int.S
  val neg : t -> t
  val not : t -> t
  val pow : t -> t -> t

  val bitwidth : int
  val of_big_int : Big_int.big_int -> t (* wrapping *)
  val to_big_int : t -> Big_int.big_int (* returns natural numbers *)
end

module MakeWord (Rep : WordRepType) : WordType =
struct
  module WasmInt = Wasm.Int.Make (Rep)
  include WasmInt
  let neg w = sub zero w
  let not w = xor w (of_int_s (-1))
  let one = of_int_u 1
  let rec pow x y =
    if y = zero then
      one
    else if and_ y one = zero then
      pow (mul x x) (shr_u y one)
    else
      mul x (pow x (sub y one))

  let bitwidth = Rep.bitwidth
  let of_big_int = Rep.of_big_int
  let to_big_int = Rep.to_big_int
end

module Word8Rep  = MakeWord (Int8Rep)
module Word16Rep = MakeWord (Int16Rep)
module Word32Rep = MakeWord (Int32Rep)
module Word64Rep = MakeWord (Int64Rep)

module type FloatType =
sig
  include Wasm.Float.S
  val rem : t -> t -> t
  val pow : t -> t -> t
  val to_pretty_string : t -> string
end

module MakeFloat(WasmFloat : Wasm.Float.S) =
struct
  include WasmFloat
  let rem x y = of_float (Float.rem (to_float x) (to_float y))
  let pow x y = of_float (to_float x ** to_float y)
  let to_pretty_string w = group_num (WasmFloat.to_string w)
  let to_string = to_pretty_string
end

module Float = MakeFloat(Wasm.F64)


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

module Int : NumType with type t = Big_int.big_int =
struct
  open Big_int
  type t = big_int
  let signed = true
  let zero = zero_big_int
  let one = unit_big_int
  let sub = sub_big_int
  let abs = abs_big_int
  let neg = minus_big_int
  let add = add_big_int
  let mul = mult_big_int
  let div a b =
    let q, m = quomod_big_int a b in
    if sign_big_int m * sign_big_int a >= 0 then q
    else if sign_big_int q = 1 then pred_big_int q else succ_big_int q
  let rem a b =
    let q, m = quomod_big_int a b in
    let sign_m = sign_big_int m in
    if sign_m * sign_big_int a >= 0 then m
    else
    let abs_b = abs_big_int b in
    if sign_m = 1 then sub_big_int m abs_b else add_big_int m abs_b
  let eq = eq_big_int
  let ne x y = not (eq x y)
  let lt = lt_big_int
  let gt = gt_big_int
  let le = le_big_int
  let ge = ge_big_int
  let compare = compare_big_int
  let to_int i = int_of_big_int i
  let of_int = big_int_of_int
  let to_int32 = int32_of_big_int
  let of_int32 = big_int_of_int32
  let to_int64 = int64_of_big_int
  let of_int64 = big_int_of_int64
  let of_big_int i = i
  let to_big_int i = i
  let to_pretty_string i = group_num (string_of_big_int i)
  let to_string = to_pretty_string
  let of_string s =
    big_int_of_string (String.concat "" (String.split_on_char '_' s))

  let max_int = big_int_of_int max_int

  let pow x y =
    if gt y max_int
    then raise (Invalid_argument "Int.pow")
    else power_big_int_positive_int x (to_int y)
end

module Nat : NumType with type t = Big_int.big_int =
struct
  include Int
  let signed = false
  let of_big_int i =
    if ge i zero then i else raise (Invalid_argument "Nat.of_big_int")
  let sub x y =
    let z = Int.sub x y in
    if ge z zero then z else raise (Invalid_argument "Nat.sub")
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

module Ranged
  (Rep : NumType)
  (WordRep : WordType)
  : BitNumType =
struct
  let to_word i = WordRep.of_big_int (Rep.to_big_int i)
  let from_word i =
    let n = WordRep.to_big_int i in
    let n' =
      let open Big_int in
      if Rep.signed && le_big_int (power_int_positive_int 2 (WordRep.bitwidth - 1)) n
      then sub_big_int n (power_int_positive_int 2 (WordRep.bitwidth))
      else n
    in
    Rep.of_big_int n'

  let check i =
    if Rep.eq (from_word (to_word i)) i
    then i
    else raise (Invalid_argument "value out of bounds")

  include Rep
  (* bounds-checking operations *)
  let neg a = let res = Rep.neg a in check res
  let abs a = let res = Rep.abs a in check res
  let add a b = let res = Rep.add a b in check res
  let sub a b = let res = Rep.sub a b in check res
  let mul a b = let res = Rep.mul a b in check res
  let div a b = let res = Rep.div a b in check res
  let pow a b = let res = Rep.pow a b in check res
  let of_int i = let res = Rep.of_int i in check res
  let of_int32 i = let res = Rep.of_int32 i in check res
  let of_int64 i = let res = Rep.of_int64 i in check res
  let of_big_int i = let res = Rep.of_big_int i in check res
  let of_string s = let res = Rep.of_string s in check res

  let on_word op a = from_word (op (to_word a))
  let on_words op a b = from_word (op (to_word a) (to_word b))

  (* bit-wise operations *)
  let not = on_word WordRep.not
  let popcnt = on_word WordRep.popcnt
  let clz = on_word WordRep.clz
  let ctz = on_word WordRep.ctz

  let and_ = on_words WordRep.and_
  let or_ = on_words WordRep.or_
  let xor = on_words WordRep.xor
  let shl = on_words WordRep.shl
  let shr = on_words (if Rep.signed then WordRep.shr_s else WordRep.shr_u)
  let rotl = on_words WordRep.rotl
  let rotr = on_words WordRep.rotr


  (* wrapping operations *)
  let wrapping_of_big_int i = from_word (WordRep.of_big_int i)

  let wadd = on_words WordRep.add
  let wsub = on_words WordRep.sub
  let wmul = on_words WordRep.mul
  let wpow a b =
    if Rep.ge b Rep.zero
    then on_words WordRep.pow a b
    else raise (Invalid_argument "negative exponent")
end

module Nat8 = Ranged (Nat) (Word8Rep)
module Nat16 = Ranged (Nat) (Word16Rep)
module Nat32 = Ranged (Nat) (Word32Rep)
module Nat64 = Ranged (Nat) (Word64Rep)

module Int_8 = Ranged (Int) (Word8Rep)
module Int_16 = Ranged (Int) (Word16Rep)
module Int_32 = Ranged (Int) (Word32Rep)
module Int_64 = Ranged (Int) (Word64Rep)
