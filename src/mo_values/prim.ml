(* Primitives *)
open Mo_types

open Value
open Numerics

(* This function raises `Invalid_argument` exception, catch it in the call site
   to convert to trap *)
let as_big_int = function
  | Type.Nat -> fun v -> Nat.to_big_int (as_int v)
  | Type.Int -> fun v -> Int.to_big_int (as_int v)
  | Type.Nat8 -> fun v -> Nat8.to_big_int (as_nat8 v)
  | Type.Nat16 -> fun v -> Nat16.to_big_int (as_nat16 v)
  | Type.Nat32 -> fun v -> Nat32.to_big_int (as_nat32 v)
  | Type.Nat64 -> fun v -> Nat64.to_big_int (as_nat64 v)
  | Type.Int8 -> fun v -> Int_8.to_big_int (as_int8 v)
  | Type.Int16 -> fun v -> Int_16.to_big_int (as_int16 v)
  | Type.Int32 -> fun v -> Int_32.to_big_int (as_int32 v)
  | Type.Int64 -> fun v -> Int_64.to_big_int (as_int64 v)
  | Type.Char -> fun v -> Big_int.big_int_of_int (as_char v)
  | t -> raise (Invalid_argument ("Value.as_big_int: " ^ Type.string_of_typ (Type.Prim t)))

(* This function raises `Invalid_argument` exception, catch it in the call site
   to convert to trap *)
let of_big_int_trap = function
  | Type.Nat -> fun i -> Int (Nat.of_big_int i)
  | Type.Int -> fun i -> Int (Int.of_big_int i)
  | Type.Nat8 -> fun i -> Nat8 (Nat8.of_big_int i)
  | Type.Nat16 -> fun i -> Nat16 (Nat16.of_big_int i)
  | Type.Nat32 -> fun i -> Nat32 (Nat32.of_big_int i)
  | Type.Nat64 -> fun i -> Nat64 (Nat64.of_big_int i)
  | Type.Int8 -> fun i -> Int8 (Int_8.of_big_int i)
  | Type.Int16 -> fun i -> Int16 (Int_16.of_big_int i)
  | Type.Int32 -> fun i -> Int32 (Int_32.of_big_int i)
  | Type.Int64 -> fun i -> Int64 (Int_64.of_big_int i)
  | Type.Char -> fun i ->
    let i = Big_int.int_of_big_int i in
    if i < 0xD800 || i >= 0xE000 && i < 0x110000 then Char i else raise (Invalid_argument "character value out of bounds")
  | t -> raise (Invalid_argument ("Value.of_big_int_trap: " ^ Type.string_of_typ (Type.Prim t)))

(* This function raises `Invalid_argument` exception, catch it in the call site
   to convert to trap *)
let of_big_int_wrap = function
  | Type.Nat8 -> fun i -> Nat8 (Nat8.wrapping_of_big_int i)
  | Type.Nat16 -> fun i -> Nat16 (Nat16.wrapping_of_big_int i)
  | Type.Nat32 -> fun i -> Nat32 (Nat32.wrapping_of_big_int i)
  | Type.Nat64 -> fun i -> Nat64 (Nat64.wrapping_of_big_int i)
  | Type.Int8 -> fun i -> Int8 (Int_8.wrapping_of_big_int i)
  | Type.Int16 -> fun i -> Int16 (Int_16.wrapping_of_big_int i)
  | Type.Int32 -> fun i -> Int32 (Int_32.wrapping_of_big_int i)
  | Type.Int64 -> fun i -> Int64 (Int_64.wrapping_of_big_int i)
  | t -> raise (Invalid_argument ("Value.of_big_int_wrap: " ^ Type.string_of_typ (Type.Prim t)))

(*
Wrapping numeric conversions are all specified uniformly by going through bigint
*)

type trap = { trap : 'a. string -> 'a  }

(* Trapping conversions (the num_conv_t1_t2 prim used in prelude/prelude.ml) *)
let num_conv_trap_prim trap t1 t2 =
  let module T = Type in
  match t1, t2 with
  | T.Nat, T.(T.Nat8|Nat16|Nat32|Nat64)
  | T.Int, T.(Int8|Int16|Int32|Int64)
  | T.(Nat8|Nat16|Nat32|Nat64), T.Nat
  | T.(Int8|Int16|Int32|Int64), T.Int
  | T.Nat8, T.Nat16
  | T.Nat16, T.Nat32
  | T.Nat32, T.Nat64
  | T.Nat64, T.Nat32
  | T.Nat32, T.Nat16
  | T.Nat16, T.Nat8
  | T.Int8, T.Int16
  | T.Int16, T.Int32
  | T.Int32, T.Int64
  | T.Int64, T.Int32
  | T.Int32, T.Int16
  | T.Int16, T.Int8
  | T.Nat32, T.Char
  -> fun v -> (try of_big_int_trap t2 (as_big_int t1 v)
               with Invalid_argument msg -> trap.trap msg)
  | T.Float, T.Int64 -> fun v -> Int64 (Int_64.of_big_int (bigint_of_double (as_float v)))
  | T.Int64, T.Float -> fun v -> Float (Wasm.F64_convert.convert_i64_s (Big_int.int64_of_big_int (Int_64.to_big_int (as_int64 v))))

  | T.Float, T.Int -> fun v -> Int (Int.of_big_int (bigint_of_double (as_float v)))
  | T.Int, T.Float -> fun v -> Float (Wasm.F64.of_float (Big_int.float_of_big_int (Int.to_big_int (as_int v))))

  | T.Float, T.Float32 ->
    fun v -> Float32 (Float32.of_float (Float.to_float (as_float v)))
  | T.Float32, T.Float -> fun v -> Float (Float.of_float (Float32.to_float (as_float32 v)))

  | t1, t2 -> trap.trap T.("Value.num_conv_trap_prim: " ^ string_of_typ (Prim t1) ^ string_of_typ (Prim t2))

(*
It is the responsibility of prelude/prelude.ml to define num_wrap_t1_t2 only
for suitable types t1 and t2
*)
let num_conv_wrap_prim trap t1 t2 =
  fun v -> try of_big_int_wrap t2 (as_big_int t1 v)
           with Invalid_argument msg -> trap.trap msg

let prim trap =
  let via_float f v = Float.(Float (of_float (f (to_float (as_float v))))) in
  let via_float2 f v w = Float.(Float (of_float (f (to_float (as_float v)) (to_float (as_float w))))) in
  let unpack_nat8 v = Nat8.to_int (as_nat8 v) in
  let float_formatter prec : int -> float -> string =
    let open Printf in
    function
    | 0 -> sprintf "%.*f" prec
    | 1 -> sprintf "%.*e" prec
    | 2 -> sprintf "%.*g" prec
    | 3 -> sprintf "%.*h" prec
    | _ -> fun _ -> trap.trap "float_formatter: unrecognised mode" in
  function
  | "abs" -> fun _ v k -> k (Int (Nat.abs (as_int v)))
  | "fabs" -> fun _ v k -> k (Float (Float.abs (as_float v)))
  | "fsqrt" -> fun _ v k -> k (Float (Float.sqrt (as_float v)))
  | "fceil" -> fun _ v k -> k (Float (Float.ceil (as_float v)))
  | "ffloor" -> fun _ v k -> k (Float (Float.floor (as_float v)))
  | "ftrunc" -> fun _ v k -> k (Float (Float.trunc (as_float v)))
  | "fnearest" -> fun _ v k -> k (Float (Float.nearest (as_float v)))
  | "fmin" -> fun _ v k ->
    (match Value.as_tup v with
     | [a; b] -> k (Float (Float.min (as_float a) (as_float b)))
     | _ -> assert false)
  | "fmax" -> fun _ v k ->
    (match Value.as_tup v with
     | [a; b] -> k (Float (Float.max (as_float a) (as_float b)))
     | _ -> assert false)
  | "fcopysign" -> fun _ v k ->
    (match Value.as_tup v with
     | [a; b] -> k (Float (Float.copysign (as_float a) (as_float b)))
     | _ -> assert false)
  | "Float->Text" -> fun _ v k -> k (Text (Float.to_string (as_float v)))
  | "Float32->Text" -> fun _ v k -> k (Text (Float32.to_string (as_float32 v)))
  | "fmtFloat->Text" -> fun _ v k ->
    (match Value.as_tup v with
     | [f; prec; mode] ->
       k (Text (float_formatter (unpack_nat8 prec) (unpack_nat8 mode) Float.(to_float (as_float f))))
     | _ -> assert false)
  | "fsin" -> fun _ v k -> k (via_float Stdlib.sin v)
  | "fcos" -> fun _ v k -> k (via_float Stdlib.cos v)
  | "ftan" -> fun _ v k -> k (via_float Stdlib.tan v)
  | "fasin" -> fun _ v k -> k (via_float Stdlib.asin v)
  | "facos" -> fun _ v k -> k (via_float Stdlib.acos v)
  | "fatan" -> fun _ v k -> k (via_float Stdlib.atan v)
  | "fatan2" -> fun _ v k ->
    (match Value.as_tup v with
     | [y; x] -> k (via_float2 Stdlib.atan2 y x)
     | _ -> assert false)
  | "fexp" -> fun _ v k -> k (via_float Stdlib.exp v)
  | "flog" -> fun _ v k -> k (via_float Stdlib.log v)
  (* TODO: refine exotic cases below to catch more errors *)
  | "popcntInt8" | "popcntInt16" | "popcntInt32" | "popcntInt64"
  | "popcnt8" | "popcnt16" | "popcnt32" | "popcnt64" ->
     fun _ v k ->
     k (match v with
        | Nat8  w -> Nat8  (Nat8. popcnt w)
        | Nat16 w -> Nat16 (Nat16.popcnt w)
        | Nat32 w -> Nat32 (Nat32.popcnt w)
        | Nat64 w -> Nat64 (Nat64.popcnt w)
        | Int8  w -> Int8  (Int_8. popcnt w)
        | Int16 w -> Int16 (Int_16.popcnt w)
        | Int32 w -> Int32 (Int_32.popcnt w)
        | Int64 w -> Int64 (Int_64.popcnt w)
        | _ -> failwith "popcnt")
  | "clzInt8" | "clzInt16" | "clzInt32" | "clzInt64"
  | "clz8" | "clz16" | "clz32" | "clz64" ->
     fun _ v k ->
     k (match v with
        | Nat8  w -> Nat8  (Nat8. clz w)
        | Nat16 w -> Nat16 (Nat16.clz w)
        | Nat32 w -> Nat32 (Nat32.clz w)
        | Nat64 w -> Nat64 (Nat64.clz w)
        | Int8  w -> Int8  (Int_8. clz w)
        | Int16 w -> Int16 (Int_16.clz w)
        | Int32 w -> Int32 (Int_32.clz w)
        | Int64 w -> Int64 (Int_64.clz w)
        | _ -> failwith "clz")
  | "ctzInt8" | "ctzInt16" | "ctzInt32" | "ctzInt64"
  | "ctz8" | "ctz16" | "ctz32" | "ctz64" ->
     fun _ v k ->
     k (match v with
        | Nat8  w -> Nat8  (Nat8. ctz w)
        | Nat16 w -> Nat16 (Nat16.ctz w)
        | Nat32 w -> Nat32 (Nat32.ctz w)
        | Nat64 w -> Nat64 (Nat64.ctz w)
        | Int8  w -> Int8  (Int_8. ctz w)
        | Int16 w -> Int16 (Int_16.ctz w)
        | Int32 w -> Int32 (Int_32.ctz w)
        | Int64 w -> Int64 (Int_64.ctz w)
        | _ -> failwith "ctz")
  | "btstInt8" | "btstInt16" | "btstInt32" | "btstInt64"
  | "btst8" | "btst16" | "btst32" | "btst64" ->
     fun _ v k ->
     let w, a = as_pair v
     in k (match w with
           | Nat8  y -> Nat8  Nat8. (and_ y (shl one (as_nat8  a)))
           | Nat16 y -> Nat16 Nat16.(and_ y (shl one (as_nat16 a)))
           | Nat32 y -> Nat32 Nat32.(and_ y (shl one (as_nat32 a)))
           | Nat64 y -> Nat64 Nat64.(and_ y (shl one (as_nat64 a)))
           | Int8  y -> Int8  Int_8. (and_ y (shl one (as_int8  a)))
           | Int16 y -> Int16 Int_16.(and_ y (shl one (as_int16 a)))
           | Int32 y -> Int32 Int_32.(and_ y (shl one (as_int32 a)))
           | Int64 y -> Int64 Int_64.(and_ y (shl one (as_int64 a)))
           | _ -> failwith "btst")

  | "lsh_Nat" -> fun _ v k ->
    (match as_tup v with
     | [x; shift] -> k (Int Numerics.Int.(mul (as_int x) (pow (of_int 2) (of_big_int (Nat32.to_big_int (as_nat32 shift))))))
     | _ -> failwith "lsh_Nat")
  | "rsh_Nat" -> fun _ v k ->
    (match as_tup v with
     | [x; shift] -> k (Int Numerics.Int.(div (as_int x) (pow (of_int 2) (of_big_int (Nat32.to_big_int (as_nat32 shift))))))
     | _ -> failwith "rsh_Nat")

  | "explode_Nat16" -> fun _ v k ->
    let n, ff = as_nat16 v, Nat16.(of_int 0xFF) in
    let byte_at p = Nat8 (Nat16.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup [byte_at 8; byte_at 0])
  | "explode_Int16" -> fun _ v k ->
    let n, ff = as_int16 v, Int_16.(of_int 0xFF) in
    let byte_at p = Nat8 (Int_16.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup [byte_at 8; byte_at 0])
  | "explode_Nat32" -> fun _ v k ->
    let n, ff = as_nat32 v, Nat32.(of_int 0xFF) in
    let byte_at p = Nat8 (Nat32.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup (List.map byte_at [24; 16; 8; 0]))
  | "explode_Int32" -> fun _ v k ->
    let n, ff = as_int32 v, Int_32.(of_int 0xFF) in
    let byte_at p = Nat8 (Int_32.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup (List.map byte_at [24; 16; 8; 0]))
  | "explode_Nat64" -> fun _ v k ->
    let n, ff = as_nat64 v, Nat64.(of_int 0xFF) in
    let byte_at p = Nat8 (Nat64.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup (List.map byte_at [56; 48; 40; 32; 24; 16; 8; 0]))
  | "explode_Int64" -> fun _ v k ->
    let n, ff = as_int64 v, Int_64.(of_int 0xFF) in
    let byte_at p = Nat8 (Int_64.(shr n (of_int p) |> and_ ff |> to_int) |> Nat8.of_int) in
    k (Tup (List.map byte_at [56; 48; 40; 32; 24; 16; 8; 0]))

  | "conv_Char_Text" -> fun _ v k -> let str = match as_char v with
                                          | c when c <= 0o177 -> String.make 1 (Char.chr c)
                                          | code -> Lib.Utf8.encode [code]
                               in k (Text str)
  | "print" -> fun _ v k -> Printf.printf "%s\n%!" (as_text v); k unit
  | "trap" -> fun _ v k -> trap.trap ("explicit trap: " ^ (as_text v))
  | "rts_in_upgrade" ->
    fun _ v k -> as_unit v; k (Bool false) (* no upgrades in interpreters *)
  | "rts_version" -> fun _ v k -> as_unit v; k (Text "0.1")
  | (  "rts_memory_size"
     | "rts_heap_size"
     | "rts_total_allocation"
     | "rts_reclaimed"
     | "rts_max_live_size"
     | "rts_callback_table_count"
     | "rts_callback_table_size"
     | "rts_mutator_instructions"
     | "rts_collector_instructions"
     | "rts_lifetime_instructions"
     | "rts_upgrade_instructions") ->
        fun _ v k -> as_unit v; k (Int (Int.of_int 0))
  | "time" -> fun _ v k -> as_unit v; k (Value.Nat64 (Numerics.Nat64.of_int 42))
  | "deadline" -> fun _ v k -> as_unit v; k (Value.Nat64 Numerics.Nat64.zero)
  | "idlHash" -> fun _ v k ->
    let s = as_text v in
    k (Nat32 (Nat32.wrapping_of_big_int (Big_int.big_int_of_int32 (Lib.Uint32.to_int32 (Idllib.IdlHash.idl_hash s)))))
  | "crc32Hash" -> fun _ v k -> let s = as_blob v in
    let i = Optint.(to_int32 (Checkseum.Crc32.digest_string s 0 (String.length s) zero)) in
    k (Nat32 (Nat32.wrapping_of_big_int (Big_int.big_int_of_int32 i)))
  | "array_len" -> fun _ v k ->
    k (Int (Int.of_int (Array.length (Value.as_array v))))
  | "blob_size" -> fun _ v k ->
    k (Int (Nat.of_int (String.length (Value.as_blob v))))
  | "blob_vals_iter" -> fun _ v k ->
    let s = String.to_seq (Value.as_blob v) in
    let valuation b = Nat8 (Nat8.of_int (Char.code b)) in
    k (Iter (ref (Seq.map valuation s)))
  | "blob_iter_done" | "text_iter_done" -> fun _ v k ->
    let i = Value.as_iter v in
    k (Bool (!i () = Seq.Nil))
  | "blob_iter_next" | "text_iter_next" -> fun _ v k ->
    let i = Value.as_iter v in
    begin match !i () with
    | Seq.Nil -> assert false
    | Seq.Cons (v, vs) -> i := vs; k v
    end
  | "text_len" -> fun _ v k ->
    k (Int (Nat.of_int (List.length (Lib.Utf8.decode (Value.as_text v)))))
  | "text_lowercase" ->
     fun _ v k ->
     k (Text (String.lowercase_ascii (Value.as_text v))) (* TODO -- use Unicode here. *)
  | "text_uppercase" -> fun _ v k ->
     k (Text (String.uppercase_ascii (Value.as_text v))) (* TODO -- use Unicode here. *)
  | "text_compare" -> fun _ v k ->
    (match Value.as_tup v with
     | [a; b] -> k (Int8 (Int_8.of_int
                            (let a, b = Value.as_text a, Value.as_text b in
                             if a = b then 0 else if a < b then -1 else 1)))
     | _ -> assert false)
  | "blob_compare" -> fun _ v k ->
    (match Value.as_tup v with
     | [a; b] -> k (Int8 (Int_8.of_int
                            (let a, b = Value.as_blob a, Value.as_blob b in
                             if a = b then 0 else if a < b then -1 else 1)))
     | _ -> assert false)
  | "text_iter" -> fun _ v k ->
    let s = Lib.Utf8.decode (Value.as_text v) in
    let i = Seq.map (fun c -> Char c) (List.to_seq s) in
    k (Iter (ref i))
  | "Array.init" -> fun _ v k ->
    (match Value.as_tup v with
    | [len; x] ->
      k (Array (Array.init (Int.to_int (as_int len)) (fun _ -> Mut (ref x))))
    | _ -> assert false
    )
  | "Array.tabulate" -> fun c v k ->
    (* TODO: optimize these (https://github.com/dfinity/motoko/pull/5256#discussion_r2143573548) *)
    (match Value.as_tup v with
    | [len; g] ->
      let len_nat = Int.to_int (as_int len) in
      let (_, g') = Value.as_func g in
      let rec go prefix k i =
        if i == len_nat
        then k (Array (Array.of_list (prefix [])))
        else g' c (Int (Int.of_int i)) (fun x -> go (fun tl -> prefix (x::tl)) k (i + 1))
      in go (fun xs -> xs) k 0
    | _ -> assert false
    )
  | "Array.tabulateVar" -> fun c v k ->
    (match Value.as_tup v with
    | [len; g] ->
      let len_nat = Int.to_int (as_int len) in
      let (_, g') = Value.as_func g in
      let rec go prefix k i =
        if i == len_nat
        then k (Array (Array.of_list (prefix [])))
        else g' c (Int (Int.of_int i)) (fun x -> go (fun tl -> prefix (Mut (ref x)::tl)) k (i + 1))
      in go (fun xs -> xs) k 0
    | _ -> assert false
    )


  | "cast"
  | "blobOfPrincipal"
  | "principalOfBlob"
  | "principalOfActor"
  | "actorOfPrincipal" -> fun _ v k -> k v

  | "blobToArray" -> fun _ v k ->
    k (Array (Array.of_seq (Seq.map (fun c ->
      Nat8 (Nat8.of_int (Char.code c))
    ) (String.to_seq (Value.as_blob v)))))
  | "blobToArrayMut" -> fun _ v k ->
    k (Array (Array.of_seq (Seq.map (fun c ->
      Mut (ref (Nat8 (Nat8.of_int (Char.code c))))
    ) (String.to_seq (Value.as_blob v)))))
  | "arrayToBlob" -> fun _ v k ->
    k (Blob (String.of_seq (Seq.map (fun v ->
      Char.chr (Nat8.to_int (Value.as_nat8 v))
    ) (Array.to_seq (Value.as_array v)))))
  | "arrayMutToBlob" -> fun _ v k ->
    k (Blob (String.of_seq (Seq.map (fun v ->
      Char.chr (Nat8.to_int (Value.as_nat8 !(Value.as_mut v)))
    ) (Array.to_seq (Value.as_array v)))))

  (* calls never fail in the interpreter *)
  | "call_perform_status" -> fun _ v k -> k (Nat32 Nat32.zero)
  | "call_perform_message" -> fun _ v k -> k (Value.Text "")

  | p when Lib.String.chop_prefix "num_conv" p <> None ->
    begin match String.split_on_char '_' p with
    | [_;_;s1;s2] ->
      let p1 = Type.prim s1 in
      let p2 = Type.prim s2 in
      fun env v k -> k (num_conv_trap_prim trap p1 p2 v)
    | _ -> assert false
    end

  | p when Lib.String.chop_prefix "num_wrap" p <> None ->
    begin match String.split_on_char '_' p with
    | [_;_;s1;s2] ->
      let p1 = Type.prim s1 in
      let p2 = Type.prim s2 in
      fun env v k -> k (num_conv_wrap_prim trap p1 p2 v)
    | _ -> assert false
    end

  | "char_to_upper" ->
      fun _ v k ->
        begin match Uucp.Case.Map.to_upper (Uchar.of_int (as_char v)) with
        | `Uchars [c] -> k (Char (Uchar.to_int c))
        | `Uchars _ ->
            (* RTS implementation of to_upper returns the input for characters
               that map to multiple characters in uppercase versions, so to be
               in sync with that we do the same here *)
            k v
        | `Self -> k v
        end

  | "char_to_lower" ->
      fun _ v k ->
        begin match Uucp.Case.Map.to_lower (Uchar.of_int (as_char v)) with
        | `Uchars [c] -> k (Char (Uchar.to_int c))
        | `Uchars _ -> k v (* same as above, in char_to_upper *)
        | `Self -> k v
        end

  | "char_is_whitespace" ->
      fun _ v k -> k (Bool (Uucp.White.is_white_space (Uchar.of_int (as_char v))))

  | "char_is_lowercase" ->
      fun _ v k -> k (Bool (Uucp.Case.is_lower (Uchar.of_int (as_char v))))

  | "char_is_uppercase" ->
      fun _ v k -> k (Bool (Uucp.Case.is_upper (Uchar.of_int (as_char v))))

  | "char_is_alphabetic" ->
      fun _ v k -> k (Bool (Uucp.Alpha.is_alphabetic (Uchar.of_int (as_char v))))

  | "decodeUtf8" ->
      fun _ v k ->
        let s = as_blob v in
        begin match Lib.Utf8.decode s with
          | _ -> k (Opt (Text s))
          | exception Lib.Utf8.Utf8 -> k Null
        end

  | "encodeUtf8" ->
      fun _ v k -> k (Blob (as_text v))

  | "is_controller" ->
      fun _ v k -> k (Bool false)

  | "canister_version" ->
      fun _ v k -> as_unit v; k (Nat64 (Numerics.Nat64.of_int 42))

  | "canister_subnet" ->
      fun _ v k -> as_unit v; k (Blob "")

  | "root_key" ->
      fun _ v k -> as_unit v; k (Blob "")

  | "canister_self" ->
      fun _ v k -> as_unit v; k (Blob "")

  (* fake *)
  | "setCandidLimits" ->
      fun _ v k -> k unit
  | "getCandidLimits" ->
      fun _ v k -> k (Tup Numerics.Nat32.[
        Nat32 zero; Nat32 zero; Nat32 zero])

  | "setCandidTypeLimits" ->
      fun _ v k -> k unit
  | "getCandidTypeLimits" ->
      fun _ v k -> k (Tup Numerics.Nat32.[
        Nat32 zero; Nat32 zero])

  | "alloc_weak_ref" ->
     fun _ v k ->
      (* TODO: model trapping on non-scalars *)
       let w = Weak.create 1 in
       Weak.set w 0 (Some v);
       k (Weak w)

  | "weak_get" ->
     fun _ v k ->
      (let w = as_weak v in
       match Weak.get w 0 with
       | Some v -> k (Opt v)
       | None -> k (Null))

  | "weak_ref_is_live" ->
     fun _ v k ->
       let w = as_weak v in
       k (Bool (Weak.check w 0))

  | "env_var_names" ->
     fun _ v k ->
       k (Array (Array.of_list []))

  | "env_var" ->
     fun _ v k ->
       k Null

  | s -> trap.trap ("Value.prim: " ^ s)
