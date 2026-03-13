type big_int

val zero_big_int : big_int
val unit_big_int : big_int

val minus_big_int : big_int -> big_int
val abs_big_int : big_int -> big_int
val succ_big_int : big_int -> big_int
val pred_big_int : big_int -> big_int

val add_big_int : big_int -> big_int -> big_int
val sub_big_int : big_int -> big_int -> big_int
val mult_big_int : big_int -> big_int -> big_int
val div_big_int : big_int -> big_int -> big_int
val mod_big_int : big_int -> big_int -> big_int
val quomod_big_int : big_int -> big_int -> big_int * big_int

val sign_big_int : big_int -> int
val compare_big_int : big_int -> big_int -> int

val eq_big_int : big_int -> big_int -> bool
val lt_big_int : big_int -> big_int -> bool
val gt_big_int : big_int -> big_int -> bool
val le_big_int : big_int -> big_int -> bool
val ge_big_int : big_int -> big_int -> bool

val int_of_big_int : big_int -> int
val big_int_of_int : int -> big_int

val int32_of_big_int : big_int -> int32
val big_int_of_int32 : int32 -> big_int

val int64_of_big_int : big_int -> int64
val big_int_of_int64 : int64 -> big_int

val float_of_big_int : big_int -> float

val string_of_big_int : big_int -> string
val big_int_of_string : string -> big_int

val power_int_positive_int : int -> int -> big_int
val power_big_int_positive_int : big_int -> int -> big_int

val shift_left_big_int : big_int -> int -> big_int
val shift_right_big_int : big_int -> int -> big_int

val is_int_big_int : big_int -> bool
