open Mo_types
open Wasm_exts.Ast
open Wasm_exts.Types

let nr x = Wasm.Source.{ it = x; at = Wasm.Source.no_region }

module Backend64 : Backend_intf.S = struct
  type t = int64
  
  let to_int64 x = x
  let of_int64 x = x
  let to_int32 = Int64.to_int32
  let of_int32 = Int64.of_int32
  
  let to_string = Int64.to_string

  let zero = 0L
  let one = 1L
  let minus_one = -1L
  let add_host = Int64.add
  let sub_host = Int64.sub
  let mul_host = Int64.mul
  let div_host = Int64.div
  let compare_host = Int64.compare
  let of_int_host = Int64.of_int
  let to_int_host = Int64.to_int
  let ge_host x y = Int64.compare x y >= 0
  let lt_host x y = Int64.compare x y < 0
  let gt_host x y = Int64.compare x y > 0
  let shl_host x n = Int64.shift_left x (to_int_host n)
  let shr_s_host x n = Int64.shift_right x n
  let shr_u_host x n = Int64.shift_right_logical x n
  let and_host = Int64.logand
  let or_host = Int64.logor
  let xor_host = Int64.logxor
  let lognot_host = Int64.lognot

  let word_size = 8
  let word_size_in_bytes = 8
  
  (* 64KB pages *)
  let page_size = 65536L
  let page_size_bits = 16
  
  let ptr_skew = -1L
  let ptr_unskew = 1L
  
  let wasm_val_type = I64Type
  let wasm_idx_type = I64IndexType
  
  (* Instructions *)
  module I = Wasm.I64
  module O = I64Op
  
  let op o = Binary (Wasm_exts.Values.I64 o)
  let uop o = Unary (Wasm_exts.Values.I64 o)
  let rel o = Compare (Wasm_exts.Values.I64 o)
  
  (* Wasm align field is log2(alignment_in_bytes). For i64: log2(8) = 3 *)
  let word_align = 3

  let load ?offset ?align () =
    let offset = Option.value ~default:0L offset in
    let align = Option.value ~default:word_align align in
    Load { ty = I64Type; align; offset; sz = None }
    
  let store ?offset ?align () =
    let offset = Option.value ~default:0L offset in
    let align = Option.value ~default:word_align align in
    Store { ty = I64Type; align; offset; sz = None }
    
  let const c = Const (nr (Wasm_exts.Values.I64 c))
  
  let add = op O.Add
  let sub = op O.Sub
  let mul = op O.Mul
  let div_s = op O.DivS
  let div_u = op O.DivU
  let rem_s = op O.RemS
  let rem_u = op O.RemU
  let and_ = op O.And
  let or_ = op O.Or
  let xor = op O.Xor
  let shl = op O.Shl
  let shr_s = op O.ShrS
  let shr_u = op O.ShrU
  let rotl = op O.Rotl
  let rotr = op O.Rotr
  
  let eq = rel O.Eq
  let ne = rel O.Ne
  let lt_s = rel O.LtS
  let lt_u = rel O.LtU
  let gt_s = rel O.GtS
  let gt_u = rel O.GtU
  let le_s = rel O.LeS
  let le_u = rel O.LeU
  let ge_s = rel O.GeS
  let ge_u = rel O.GeU
  
  let eqz = Test (Wasm_exts.Values.I64 O.Eqz)
  
  let popcnt = uop O.Popcnt
  let clz = uop O.Clz
  let ctz = uop O.Ctz
  let extend_s sz = uop (O.ExtendS sz)
  
  let wrap_relop r = Compare (Wasm_exts.Values.I64 r)
  let wrap_testop t = Test (Wasm_exts.Values.I64 t)
  let wrap_ibinop b = Binary (Wasm_exts.Values.I64 b)

  let memory_size = MemorySize
  let memory_grow = MemoryGrow
  let memory_copy = MemoryCopy
  let memory_fill = MemoryFill
  let memory_init x = MemoryInit x

  module TaggingScheme = struct
    let debug = false (* should never be true in master! *)
    
    let tag_of_typ pty = Type.(
      if !Mo_config.Flags.rtti then
        match pty with
        | Nat
        | Int ->                                                                        0b10L
        | Nat64 ->                                                                    0b0100L
        | Int64 ->                                                                    0b1100L
        | Nat32 ->                                     0b01000000_00000000_00000000_00000000L
        | Int32 ->                                     0b11000000_00000000_00000000_00000000L
        | Char  ->                        0b010_00000000_00000000_00000000_00000000_00000000L
        | Nat16 ->                   0b01000000_00000000_00000000_00000000_00000000_00000000L
        | Int16 ->                   0b11000000_00000000_00000000_00000000_00000000_00000000L
        | Nat8  ->          0b01000000_00000000_00000000_00000000_00000000_00000000_00000000L
        | Int8  ->          0b11000000_00000000_00000000_00000000_00000000_00000000_00000000L
        | _  -> assert false
      else
        (* no tag *)
        match pty with
        | Nat
        | Int
        | Nat64
        | Int64
        | Nat32
        | Int32
        | Char
        | Nat16
        | Int16
        | Nat8
        | Int8 -> 0L
        | _  -> assert false)

    let unit_tag =
      if !Mo_config.Flags.rtti then
        (* all tag, no payload (none needed) *)
        0b01000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000L
      else
        (* no tag *)
        0L

    (* Number of payload bits in compact representation, including any sign *)
    let ubits_of pty = Type.(
      if !Mo_config.Flags.rtti then
        match pty with
        | Nat | Int     -> 62
        | Nat64 | Int64 -> 60
        | Nat32 | Int32 -> 32
        | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
        | Nat16 | Int16 -> 16
        | Nat8  | Int8  ->  8
        | _ -> assert false
      else
        match pty with
        | Nat   | Int   -> 63
        | Nat64 | Int64 -> 63
        | Nat32 | Int32 -> 32
        | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
        | Nat16 | Int16 -> 16
        | Nat8  | Int8  ->  8
        | _ -> assert false)
  end
end

module Backend32 : Backend_intf.S = struct
  type t = int32
  
  let to_int64 = Int64.of_int32
  let of_int64 = Int64.to_int32
  let to_int32 x = x
  let of_int32 x = x
  
  let to_string = Int32.to_string

  let zero = 0l
  let one = 1l
  let minus_one = -1l
  let add_host = Int32.add
  let sub_host = Int32.sub
  let mul_host = Int32.mul
  let div_host = Int32.div
  let compare_host = Int32.compare
  let of_int_host = Int32.of_int
  let to_int_host = Int32.to_int
  let ge_host x y = Int32.compare x y >= 0
  let lt_host x y = Int32.compare x y < 0
  let gt_host x y = Int32.compare x y > 0
  let shl_host x n = Int32.shift_left x (to_int_host n)
  let shr_s_host x n = Int32.shift_right x n
  let shr_u_host x n = Int32.shift_right_logical x n
  let and_host = Int32.logand
  let or_host = Int32.logor
  let xor_host = Int32.logxor
  let lognot_host = Int32.lognot

  let word_size = 4
  let word_size_in_bytes = 4
  
  (* 64KB pages *)
  let page_size = 65536l
  let page_size_bits = 16
  
  let ptr_skew = -1l
  let ptr_unskew = 1l
  
  let wasm_val_type = I32Type
  let wasm_idx_type = I32IndexType
  
  (* Instructions *)
  module I = Wasm.I32
  module O = I32Op
  
  let op o = Binary (Wasm_exts.Values.I32 o)
  let uop o = Unary (Wasm_exts.Values.I32 o)
  let rel o = Compare (Wasm_exts.Values.I32 o)
  
  (* Wasm align field is log2(alignment_in_bytes). For i32: log2(4) = 2 *)
  let word_align = 2

  let load ?offset ?align () =
    let offset = Option.value ~default:0L offset in
    let align = Option.value ~default:word_align align in
    Load { ty = I32Type; align; offset; sz = None }
    
  let store ?offset ?align () =
    let offset = Option.value ~default:0L offset in
    let align = Option.value ~default:word_align align in
    Store { ty = I32Type; align; offset; sz = None }
    
  let const c = Const (nr (Wasm_exts.Values.I32 c))
  
  let add = op O.Add
  let sub = op O.Sub
  let mul = op O.Mul
  let div_s = op O.DivS
  let div_u = op O.DivU
  let rem_s = op O.RemS
  let rem_u = op O.RemU
  let and_ = op O.And
  let or_ = op O.Or
  let xor = op O.Xor
  let shl = op O.Shl
  let shr_s = op O.ShrS
  let shr_u = op O.ShrU
  let rotl = op O.Rotl
  let rotr = op O.Rotr
  
  let eq = rel O.Eq
  let ne = rel O.Ne
  let lt_s = rel O.LtS
  let lt_u = rel O.LtU
  let gt_s = rel O.GtS
  let gt_u = rel O.GtU
  let le_s = rel O.LeS
  let le_u = rel O.LeU
  let ge_s = rel O.GeS
  let ge_u = rel O.GeU
  
  let eqz = Test (Wasm_exts.Values.I32 O.Eqz)
  
  let popcnt = uop O.Popcnt
  let clz = uop O.Clz
  let ctz = uop O.Ctz
  let extend_s sz = uop (O.ExtendS sz)
  
  let wrap_relop r = Compare (Wasm_exts.Values.I32 r)
  let wrap_testop t = Test (Wasm_exts.Values.I32 t)
  let wrap_ibinop b = Binary (Wasm_exts.Values.I32 b)

  let memory_size = MemorySize
  let memory_grow = MemoryGrow
  let memory_copy = MemoryCopy
  let memory_fill = MemoryFill
  let memory_init x = MemoryInit x

  module TaggingScheme = struct
    let debug = false (* should never be true in master! *)
    
    let tag_of_typ pty = Type.(
      if !Mo_config.Flags.rtti then
        match pty with
        | Nat
        | Int ->                                    0b10l
        | Nat64 ->                                0b0100l
        | Int64 ->                                0b1100l
        | Nat32 ->                               0b01000l
        | Int32 ->                               0b11000l
        | Char  ->                        0b010_00000000l
        | Nat16 ->                   0b01000000_00000000l
        | Int16 ->                   0b11000000_00000000l
        | Nat8  ->          0b01000000_00000000_00000000l
        | Int8  ->          0b11000000_00000000_00000000l
        | _  -> assert false
      else
        (* no tag *)
        match pty with
        | Nat
        | Int
        | Nat64
        | Int64
        | Nat32
        | Int32
        | Char
        | Nat16
        | Int16
        | Nat8
        | Int8 -> 0l
        | _  -> assert false)

    let unit_tag =
      if !Mo_config.Flags.rtti then
        (* all tag, no payload (none needed) *)
        0b01000000_00000000_00000000_00000000l
      else
        (* no tag *)
        0l

    (* Number of payload bits in compact representation, including any sign *)
    let ubits_of pty = Type.(
      if !Mo_config.Flags.rtti then
        match pty with
        | Nat   | Int   -> 30
        | Nat64 | Int64 -> 28
        | Nat32 | Int32 -> 27
        | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
        | Nat16 | Int16 -> 16
        | Nat8  | Int8  ->  8
        | _ -> assert false
     else
        match pty with
        | Nat   | Int   -> 31
        | Nat64 | Int64 -> 31
        | Nat32 | Int32 -> 31
        | Char          -> 21 (* suffices for 21-bit UTF8 codepoints *)
        | Nat16 | Int16 -> 16
        | Nat8  | Int8  ->  8
        | _ -> assert false)
  end
end
