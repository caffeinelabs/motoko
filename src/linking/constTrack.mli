(* Abstract interpreter for tracking constant integral values on the Wasm operand stack.
   Uses an LRU cache keyed by stack depth to propagate constants through straight-line code.

   Iteration stops at `call` instructions and branches.
   At branches the LRU is emptied (join points handled later).
   Pure data structures throughout — suitable for bifurcation on branches.
*)

(** An integral constant on the stack *)
type const_val =
  | I32 of Int32.t
  | I64 of Int64.t

(** The LRU cache tracking stack constants. Immutable/pure. *)
type t

(** Create an empty LRU with the given capacity (number of slots). *)
val empty : int -> t

(** Process a basic block (list of instructions) starting with the given LRU state.
    Returns the LRU state at the end of the block, or [None] if a `call`, branch,
    or other terminator was encountered.
    The callback [func_type] maps a function index to its parameter and result arities:
    [func_type idx] returns [(n_params, n_results)]. *)
val process_block :
  func_type:(Int32.t -> int * int) ->
  t ->
  Wasm.Ast.instr list ->
  t option

(** Look up the constant at a given stack depth (0 = top of stack), if known. *)
val lookup : t -> int -> const_val option

(** Return all known constants as [(depth, value)] pairs, shallowest first. *)
val entries : t -> (int * const_val) list

(** Pretty-print the LRU state for debugging. *)
val dump : t -> string
