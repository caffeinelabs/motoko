(* Drop-in replacement for OCaml's Big_int module, backed by Int64.
   Used by wasm_of_ocaml builds to avoid the `num` library's C stubs.
   Sufficient for type-checking bounded integer types (Nat8..Nat64, Int8..Int64).
   Limitations:
   - Cannot represent values outside [-2^63, 2^63-1].
   - Literals > 2^63 are clamped; this means very large Nat/Nat64 literals
     will be accepted even if out of range. Acceptable for IDE type checking.
   - power_int_positive_int overflows silently for large exponents, but the
     division/modulo functions are hardened against the resulting 0. *)

type big_int = Int64.t

let zero_big_int = 0L
let unit_big_int = 1L

let minus_big_int = Int64.neg
let abs_big_int x =
  if x = Int64.min_int then Int64.max_int
  else if x < 0L then Int64.neg x
  else x
let succ_big_int x = Int64.add x 1L
let pred_big_int x = Int64.sub x 1L

let add_big_int = Int64.add
let sub_big_int = Int64.sub
let mult_big_int = Int64.mul

let sign_big_int x =
  if x > 0L then 1
  else if x < 0L then -1
  else 0

let quomod_big_int a b =
  if b = 0L then (0L, a)
  else (Int64.div a b, Int64.rem a b)

let div_big_int a b =
  if b = 0L then 0L
  else Int64.div a b

let mod_big_int a b =
  if b = 0L then a
  else
    let r = Int64.rem a b in
    if r < 0L then Int64.add r (abs_big_int b) else r

let eq_big_int (a : Int64.t) b = a = b
let lt_big_int (a : Int64.t) b = a < b
let gt_big_int (a : Int64.t) b = a > b
let le_big_int (a : Int64.t) b = a <= b
let ge_big_int (a : Int64.t) b = a >= b

let compare_big_int (a : Int64.t) b = Int64.compare a b

let int_of_big_int = Int64.to_int
let big_int_of_int = Int64.of_int

let int32_of_big_int x = Int64.to_int32 x
let big_int_of_int32 x = Int64.of_int32 x

let int64_of_big_int (x : big_int) : int64 = x
let big_int_of_int64 (x : int64) : big_int = x

let string_of_big_int = Int64.to_string

let big_int_of_string s =
  try Int64.of_string s
  with Failure _ ->
    if String.length s > 0 && s.[0] <> '-' then Int64.max_int
    else Int64.min_int

let power_int_positive_int base exp =
  let rec go acc b e =
    if e = 0 then acc
    else if e mod 2 = 0 then go acc (Int64.mul b b) (e / 2)
    else go (Int64.mul acc b) b (e - 1)
  in
  go 1L (Int64.of_int base) exp

let power_big_int_positive_int base exp =
  let rec go acc b e =
    if e = 0 then acc
    else if e mod 2 = 0 then go acc (Int64.mul b b) (e / 2)
    else go (Int64.mul acc b) b (e - 1)
  in
  go 1L base exp

let shift_left_big_int x n =
  if n >= 64 then 0L
  else if n >= 0 then Int64.shift_left x n
  else Int64.shift_right x (-n)

let shift_right_big_int x n =
  if n >= 64 then (if x < 0L then -1L else 0L)
  else if n >= 0 then Int64.shift_right x n
  else Int64.shift_left x (-n)

let float_of_big_int = Int64.to_float

let is_int_big_int x =
  let i = Int64.to_int x in
  Int64.of_int i = x
