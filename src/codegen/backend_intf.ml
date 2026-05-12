open Mo_types
open Wasm_exts.Ast
open Wasm_exts.Types

module type S = sig
  (* Host language type for words *)
  type t
  
  val to_int64 : t -> int64
  val of_int64 : int64 -> t
  val to_int32 : t -> int32
  val of_int32 : int32 -> t
  
  (* Host Language Operations *)
  val zero : t
  val one : t
  val minus_one : t
  val to_string : t -> string
  val add_host : t -> t -> t
  val sub_host : t -> t -> t
  val mul_host : t -> t -> t
  val div_host : t -> t -> t
  val compare_host : t -> t -> int
  val of_int_host : int -> t
  val to_int_host : t -> int
  val ge_host : t -> t -> bool
  val lt_host : t -> t -> bool
  val gt_host : t -> t -> bool
  val shl_host : t -> t -> t
  val shr_s_host : t -> int -> t
  val shr_u_host : t -> int -> t
  val and_host : t -> t -> t
  val or_host : t -> t -> t
  val xor_host : t -> t -> t
  val lognot_host : t -> t
  
  (* Constants *)
  val word_size : int
  val word_size_in_bytes : int
  val word_align : int  (* log2(word_size_in_bytes): 3 for 64-bit, 2 for 32-bit *)
  val page_size : t
  val page_size_bits : int
  
  val ptr_skew : t
  val ptr_unskew : t
  
  (* Wasm Types *)
  val wasm_val_type : value_type
  val wasm_idx_type : index_type
  
  (* Tagging *)
  module TaggingScheme : sig
    val debug : bool
    val tag_of_typ : Type.prim -> t
    val unit_tag : t
    val ubits_of : Type.prim -> int
  end

  (* Instruction Generators *)
  val load : ?offset:int64 -> ?align:int -> unit -> instr'
  val store : ?offset:int64 -> ?align:int -> unit -> instr'
  
  val const : t -> instr'
  
  val add : instr'
  val sub : instr'
  val mul : instr'
  val div_s : instr'
  val div_u : instr'
  val rem_s : instr'
  val rem_u : instr'
  val and_ : instr'
  val or_ : instr'
  val xor : instr'
  val shl : instr'
  val shr_s : instr'
  val shr_u : instr'
  val rotl : instr'
  val rotr : instr'
  
  val eq : instr'
  val ne : instr'
  val lt_s : instr'
  val lt_u : instr'
  val gt_s : instr'
  val gt_u : instr'
  val le_s : instr'
  val le_u : instr'
  val ge_s : instr'
  val ge_u : instr'
  
  val eqz : instr'
  
  val popcnt : instr'
  val clz : instr'
  val ctz : instr'
  val extend_s : pack_size -> instr'
  
  (* Generic wrappers: wrap an IntOp into the backend's word-size instruction *)
  val wrap_relop : IntOp.relop -> instr'
  val wrap_testop : IntOp.testop -> instr'
  val wrap_ibinop : IntOp.binop -> instr'

  (* Memory Operations *)
  val memory_size : instr'
  val memory_grow : instr'
  val memory_copy : instr'
  val memory_fill : instr'
  val memory_init : var -> instr'

end
