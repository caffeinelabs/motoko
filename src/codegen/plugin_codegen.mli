(* Versioned, native compiler extension interface. Generators run at compile
   time; their output is linked into the canister as ordinary Wasm code.
   The heap operations below share the compiler's forwarding and allocation
   barriers. A generated operation must not introduce a GC safepoint.
   Synchronous Motoko callbacks use the ordinary closure calling convention;
   collection runs only after synchronous message frames have returned.
   Generator-owned state across callbacks must use vanilla managed references.
   Compiler-owned array traversals may retain cursors under this no-safepoint
   contract. *)
open Mo_types
open Wasm_exts.Types
module G = InstrList

val abi_version : int

(** A package requires an incompatible compiler plugin ABI. *)
exception Unsupported of string

module type Backend = sig
  type env
  val compile_unboxed_const : int64 -> G.t
  val compile_const_32 : int32 -> G.t
  val compile_unboxed_zero : G.t
  val compile_unboxed_one : G.t
  val compile_add_const : int64 -> G.t
  val compile_mul_const : int64 -> G.t
  val compile_eq_const : int64 -> G.t
  val compile_comparison : Wasm_exts.Ast.I64Op.relop -> G.t
  val compile_comparison_f32 : Wasm_exts.Ast.F32Op.relop -> G.t
  val compile_comparison_f64 : Wasm_exts.Ast.F64Op.relop -> G.t
  val new_local : env -> string -> G.t * G.t
  val new_local_ : env -> value_type -> string -> G.t * G.t * int32
  val from_0_to_n : env -> (G.t -> G.t) -> G.t
  val call : env -> string -> string -> G.t
  val alloc_blob : env -> G.t -> G.t
  val unbox_float32 : env -> G.t
  val box_float32 : env -> G.t
  val buffer_size : env -> Type.typ -> G.t
  val candid_primitive : Type.typ -> int32 option
  val sort_by_hash : Type.field list -> (Lib.Uint32.t * Type.field) list
  module E : sig
    val if0 : G.t -> G.t -> G.t
    val if1 : value_type -> G.t -> G.t -> G.t
    val trap_with : env -> string -> G.t
    val then_trap_with : env -> string -> G.t
    val else_trap_with : env -> string -> G.t
  end
  module Func : sig
    type sharing = Always | Never
    val share_code : sharing -> env -> string ->
      (string * value_type) list -> value_type list ->
      (env -> G.t list -> G.t) -> G.t
    val share_code1 : sharing -> env -> string ->
      string * value_type -> value_type list -> (env -> G.t -> G.t) -> G.t
    val share_code3 : sharing -> env -> string ->
      (string * value_type) * (string * value_type) * (string * value_type) ->
      value_type list -> (env -> G.t -> G.t -> G.t -> G.t) -> G.t
  end
  module Call : sig
    (* Complete instantiated Local/Returns function type, including sequence
       arities. Evaluate closure then arguments once, and return a vanilla
       value of Type.seq results. No suspension or collection is introduced. *)
    val local : env -> Type.typ -> G.t -> G.t list -> G.t
  end
  module Compare : sig
    (* Consume two vanilla values and return unboxed -1, 0, or 1. Supported:
       Nat/Int and fixed integers, Bool, Char, Text, Blob, Principal. *)
    val scalar : env -> Type.prim -> G.t
    (* Dispatch once per operation on a compiler-proven scalar comparator.
       Arguments are evaluated once, left-to-right, as vanilla values. The
       comparator_index names the closure argument. The body receives a fresh
       function environment and argument getters: None preserves the original
       callback; Some p proves the closure implements scalar ordering p.
       This proof describes comparison only, not argument/storage types.
       Bodies are generated after ordinary functions, for materialized scalar
       kinds only. They may share helpers, but must not capture caller locals.
       Each branch leaves the declared Wasm results, including zero/multiple. *)
    val dispatch : env -> comparator_index:int -> G.t list -> value_type list ->
      (env -> Type.prim option -> G.t list -> G.t) -> G.t
  end
  module Prepared : sig
    (** Seal a completed immutable managed Blob, or borrow its representation.
        These are zero-cost operations. Ownership and the unwrapped value's
        declared type are checked at generation time, never in emitted Wasm.
        The plugin must not mutate a published payload or retain its pointer
        across calls. Use Blob helpers to resolve forwarding on each borrow. *)
    val wrap : Mo_values.Compute_plugin.Prepared.t -> G.t -> G.t
    val unwrap : Mo_values.Compute_plugin.Prepared.t -> Type.typ -> G.t -> G.t
  end
  module Blob : sig
    val len : env -> G.t
    val payload_ptr_unskewed : env -> G.t
    val of_ptr_size : env -> G.t
  end
  module Arr : sig
    (* Consume a managed array and unboxed i64 index; bounds-checked element
       load using the compiler's normal forwarding barrier. *)
    val get : env -> G.t
    val len : env -> G.t
    val unsafe_idx : env -> G.t
    (* [for_each env array length body] borrows an element cursor. Neither
        [body] nor its callees may introduce a GC safepoint. *)
    val for_each : env -> G.t -> G.t -> (G.t -> G.t) -> G.t
    (* Length and index are unboxed i64. Initialize slots before calling the
       element generator, so an early error return cannot leave invalid heap
       fields. No element generator may introduce a GC safepoint. *)
    val tabulate : env -> G.t -> (G.t -> G.t) -> G.t
    (* Mutable construction uses the same initialized slots and GC barriers. *)
    val tabulate_mut : env -> G.t -> (G.t -> G.t) -> G.t
    (* [set env array_type value_type array index value] checks types at
       generation time, evaluates all operands once left-to-right, then checks
       the unboxed i64 index and writes with the normal GC barrier. No result. *)
    val set : env -> Type.typ -> Type.typ -> G.t -> G.t -> G.t -> G.t
  end
  module Tuple : sig
    val load_n : env -> int64 -> G.t
    val from_stack : env -> int -> G.t
    val compile_unit : env -> G.t
  end
  module Object : sig
    val load_idx_raw : env -> string -> G.t
    val lit_raw : env -> (string * (unit -> G.t)) list -> G.t
    (* Typed access follows mutable-field indirections. [load] consumes the
       object. [set] evaluates object then value once, and leaves no result.
       Container shape, field mutability and value types are checked during
       generation; opaque payload types do not need a concrete representation. *)
    val load : env -> Type.typ -> string -> G.t
    val set : env -> Type.typ -> string -> Type.typ -> G.t -> G.t -> G.t
    (* All payloads are evaluated in list order before allocating the record
       and fresh mutable-field cells. The labels must match the concrete type. *)
    val lit : env -> Type.typ ->
      (string * Type.typ * (unit -> G.t)) list -> G.t
  end
  module Variant : sig
    val inject : env -> string -> G.t -> G.t
    val project : env -> G.t
    val test_is : env -> string -> G.t
    val get_variant_tag : env -> G.t
    val hash_variant_label : env -> string -> int64
  end
  module Opt : sig
    val null_lit : env -> G.t
    val inject : env -> Type.typ -> G.t -> G.t
    val is_some : env -> G.t
    val project : env -> Type.typ -> G.t
  end
  module Text : sig
    val size : env -> G.t
    val lit : env -> string -> G.t
    val of_ptr_size : env -> G.t
    val to_blob : env -> G.t
  end
  module Principal : sig
    (* Consume raw pointer and byte count. The caller validates <= 29 bytes. *)
    val of_ptr_size : env -> G.t
  end
  module Float : sig
    val unbox : env -> G.t
    val box : env -> G.t
  end
  module BoxedWord64 : sig
    val unbox : env -> Type.prim -> G.t
    val box : env -> Type.prim -> G.t
  end
  module TaggedSmallWord : sig
    val lsb_adjust : Type.prim -> G.t
    val msb_adjust : Type.prim -> G.t
    val tag : env -> Type.prim -> G.t
    val lsb_adjust_codepoint : env -> G.t
  end
  module BitTagged : sig
    val if_tagged_scalar : env -> value_type list -> G.t -> G.t -> G.t
  end
  module BigNum : sig
    val to_word64 : env -> G.t
    val from_word64 : env -> G.t
    val from_signed_word64 : env -> G.t
    val compile_add : env -> G.t
    val compile_mul : env -> G.t
    val compile_neg : env -> G.t
    (* A scalar bridge for arbitrary precision numbers; [true] means signed.
       Encoding consumes a vanilla integer and returns a Blob. Decoding
       consumes raw pointer and size of validated (S)LEB bytes. *)
    val to_leb128 : env -> bool -> G.t
    val of_leb128 : env -> bool -> G.t
  end
  module BigNumLibtommath : sig
    val fits_unsigned_bits : env -> int -> G.t
  end
  module Bool : sig
    val lit : bool -> G.t
    val from_rts_int32 : G.t
    val to_rts_int32 : G.t
  end
  module I32Leb : sig val compile_leb128_size : G.t -> G.t end
  module Strm : sig
    val name_for : string -> Type.typ list -> string
    val reserve : env -> G.t -> int64 -> G.t
    val write_byte : env -> G.t -> G.t -> G.t
    val write_word_32 : env -> G.t -> G.t -> G.t
    val write_word_leb : env -> G.t -> G.t -> G.t
    val write_bignum_leb : env -> G.t -> G.t -> G.t
    val write_bignum_sleb : env -> G.t -> G.t -> G.t
    val write_blob : env -> G.t -> G.t -> G.t
    val write_text : env -> G.t -> G.t -> G.t
  end
end

(* Managed leaves one vanilla value. Tuple n leaves n vanilla fields of a
   statically known tuple result (n = 0 or n >= 2). The compiler validates this
   claim against the normalized result type without promoting opaque bounds,
   and packs a managed tuple only when its surrounding context needs one. *)
type result_representation = Managed | Tuple of int
type generated = {code : G.t; representation : result_representation}

module type Generator = sig
  module Make (B : Backend) : sig
    (* Each getter reads one already evaluated vanilla argument. The compiler
       evaluates arguments left-to-right exactly once, using ordinary function
       sequence arities, without allocating an argument tuple. *)
    val compile : B.env -> Mo_values.Compute_plugin.specialization -> G.t list -> generated
  end
end

type runtime_function = string * value_type list * value_type list
type plugin = {
  abi : int;
  name : string;
  runtime : string option;
  types : Mo_values.Compute_plugin.Prepared.t list;
  imports : runtime_function list;
  bindings : (Mo_values.Compute_plugin.binding * (module Generator)) list;
}

type installed = { plugin : plugin; runtime_path : string option }

(** Register during --plugin loading; plugins are trusted native code. *)
val register : plugin -> unit
val load : (string -> unit) -> string -> unit
val installed : unit -> installed list
val module_name : plugin -> string
val generator : string -> (string * (module Generator)) option
