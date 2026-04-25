(* Abstract interpreter for tracking constant integral values on the Wasm operand stack.
   Uses an LRU cache keyed by stack depth to propagate constants through straight-line code.

   Continues through `call` instructions (shifting the LRU by the net stack delta).
   Stops at branches and control flow (join points handled later).
   Pure data structures throughout — suitable for bifurcation on branches.
*)

(** An integral constant on the stack *)
type const_val =
  | I32 of Int32.t
  | I64 of Int64.t
  | FromLocal of Int32.t  (** unknown value from local n; enables backprop *)

(** The LRU cache tracking stack constants. Immutable/pure. *)
type t

(** Create an empty LRU with the given capacity (number of slots). *)
val empty : int -> t

(** Process a basic block (list of instructions) starting with the given LRU state.
    Returns the LRU state at the end of the block, or [None] if a branch
    or other terminator was encountered.
    The callback [func_type] maps a function index to its parameter and result arities:
    [func_type idx] returns [(n_params, n_results)].
    The optional [on_call] callback is invoked at each [Call]/[CallIndirect] with
    the LRU state {i before} the call, the 0-based instruction index, the number
    of parameters, and the number of results.  Use this to inspect the stack at
    call sites without modifying the abstract interpreter. *)
val process_block :
  func_type:(Int32.t -> int * int) ->
  ?type_section:(Int32.t -> Wasm_exts.Types.func_type) ->
  ?on_call:(t -> int -> int -> int -> Wasm_exts.Ast.instr -> unit) ->
  t ->
  Wasm_exts.Ast.instr list ->
  t option

(** Look up the constant at a given stack depth (0 = top of stack), if known. *)
val lookup : t -> int -> const_val option

(** Return all known constants as [(depth, value)] pairs, shallowest first. *)
val entries : t -> (int * const_val) list

(** Pretty-print the LRU state for debugging. *)
val dump : t -> string

(** Physical-identity comparison for instruction phrases.

    Callers of [process_block] that wish to rewrite a particular call site
    captured by [on_call] need to relocate the exact AST node again — the
    [idx] passed to the callback is block-relative (fresh 0 at each nested
    [Block]/[Loop]/[If]), so positional keys misalign across nesting levels.

    This predicate identifies an instruction by its OCaml allocation, which
    is the invariant the linker relies on: moc's Wasm IR is tree-shaped
    (no phrase sharing, no position aliasing), so every position in a
    body owns a distinct phrase record. Use [same_instr] in preference to
    [(==)] at call sites to document the assumption. *)
val same_instr : Wasm_exts.Ast.instr -> Wasm_exts.Ast.instr -> bool
