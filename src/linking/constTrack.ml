(* Abstract interpreter for tracking constant integral values on the Wasm operand stack.
   See constTrack.mli for the interface description. *)

open Wasm.Source
open Wasm_exts.Ast

type const_val =
  | I32 of Int32.t
  | I64 of Int64.t
  | FromLocal of Int32.t  (* unknown value from local n; enables backprop *)

(* LRU entry: stack depth and the constant value *)
type entry = { depth : int; value : const_val }

(* Pure LRU: a list of entries ordered by recency (most recent first),
   with a capacity cap. *)
module LocalMap = Map.Make(Int32)

type refinement = { local_idx : Int32.t; value : const_val; sense : bool }
(* sense=true means "if TOS is true, local = value" (from Eq)
   sense=false means "if TOS is false, local = value" (from Ne) *)

type t = {
  capacity : int;
  entries : entry list;
  locals : const_val LocalMap.t;  (* known constant values of locals *)
  refinement : refinement option;  (* pending refinement from Compare *)
}

let empty capacity = { capacity; entries = []; locals = LocalMap.empty; refinement = None }

(* Phase 3: branches emit their outgoing LRU state as an effect.
   The nearest enclosing Block/If/Loop handler catches (0, lru) and either
   collects it (Block/If) or swallows it (Loop). Depths n>0 are re-raised
   as n-1 so the effect propagates de-Bruijn-style through nested blocks. *)
type _ Effect.t += May_leave : int * t -> unit Effect.t

(* Shift all depths by delta, evict entries with negative depths *)
let shift_and_evict delta lru =
  let entries =
    List.filter_map (fun e ->
      let depth = e.depth + delta in
      if depth >= 0 then Some { e with depth } else None
    ) lru.entries
  in
  { lru with entries }

(* Insert a constant at depth 0, evicting the deepest if full *)
let insert value lru =
  (* Remove any existing entry at depth 0 *)
  let entries = List.filter (fun e -> e.depth <> 0) lru.entries in
  let entry = { depth = 0; value } in
  let entries = entry :: entries in
  (* If over capacity, remove the deepest *)
  if List.length entries > lru.capacity then
    let sorted = List.sort (fun a b -> compare a.depth b.depth) entries in
    { lru with entries = List.filteri (fun i _ -> i < lru.capacity) sorted }
  else
    { lru with entries }

(* Evict the entry at depth 0 (TOS replaced by unknown value) *)
let evict_tos lru =
  { lru with entries = List.filter (fun e -> e.depth <> 0) lru.entries }

(* Maybe insert a result at depth 0; wraps in Some for |> chaining *)
let maybe_insert result lru = Some (match result with
  | Some v -> insert v lru
  | None -> lru)

(* Evict all FromLocal entries referring to local n *)
let evict_from_local n lru =
  let is_from n = function FromLocal m -> Int32.equal m n | _ -> false in
  let ents = List.filter (fun (e : entry) -> not (is_from n e.value)) lru.entries in
  let locs = LocalMap.filter (fun _k v -> not (is_from n v)) lru.locals in
  { lru with entries = ents; locals = locs }

(* Try to compute a binary i32 operation on two constants *)
let try_binop_i32 op a b =
  match a, b with
  | I32 x, I32 y -> Some (I32 (op x y))
  | _ -> None

let try_binop_i64 op a b =
  match a, b with
  | I64 x, I64 y -> Some (I64 (op x y))
  | _ -> None

let lookup lru depth =
  List.find_map (fun e -> if e.depth = depth then Some e.value else None) lru.entries

let entries lru =
  let sorted = List.sort (fun a b -> compare a.depth b.depth) lru.entries in
  List.map (fun e -> (e.depth, e.value)) sorted

let dump lru =
  let open Buffer in
  let buf = create 64 in
  add_string buf "LRU[";
  List.iter Printf.(fun (d, v) ->
    bprintf buf " %d:" d;
    (match v with
     | I32 n -> bprintf buf "i32(%ld)" n
     | I64 n -> bprintf buf "i64(%Ld)" n
     | FromLocal n -> bprintf buf "local(%ld)" n);
  ) (entries lru);
  add_string buf " ]";
  contents buf

(* Intersect two LRUs: keep only entries present in both at the same depth with the same value.
   Also intersect local maps: keep only locals that agree on value. *)
let intersect lru1 lru2 =
  let entries =
    List.filter (fun e1 ->
      List.exists (fun e2 -> e2.depth = e1.depth && e2.value = e1.value) lru2.entries
    ) lru1.entries
  in
  let locals = LocalMap.merge (fun _k v1 v2 ->
    match v1, v2 with
    | Some a, Some b when a = b -> Some a
    | _ -> None
  ) lru1.locals lru2.locals
  in
  { lru1 with entries; locals; refinement = None }

(* Run [body_proc] under a handler that catches [May_leave]:
   - depth 0 is routed to [on_zero] (collect or swallow, caller's choice);
   - depth n>0 is re-raised as n-1 so the next enclosing handler sees it
     at the correct label level.
   When [normalise] is true, a [Some lru'] return from [body_proc] is
   converted into a synthetic [May_leave (0, lru')] so fall-through and
   explicit branches-to-End merge through the same [on_zero] callback. *)
let run_under_leave_handler ~on_zero ~normalise body_proc =
  let effc : type a. a Effect.t -> ((a, unit) Effect.Deep.continuation -> unit) option = function
    | May_leave (0, lru_b) ->
      Some (fun k -> on_zero lru_b; Effect.Deep.continue k ())
    | May_leave (n, lru_b) ->
      Some (fun k ->
        May_leave (n - 1, lru_b) |> Effect.perform;
        Effect.Deep.continue k ())
    | _ -> None in
  Effect.Deep.try_with
    (fun () ->
       match body_proc () with
       | Some lru' when normalise -> May_leave (0, lru') |> Effect.perform
       | Some _ | None -> ())
    ()
    { Effect.Deep.effc = effc }

(* Fold-intersect a list of collected branch states. None if the list is
   empty (body terminated without reaching its join). *)
let join_branch_states branch_states =
  match branch_states with
  | [] -> None
  | s :: rest -> Some (List.fold_left intersect s rest)

(* De-reference a Wasm branch-target phrase to an OCaml int label depth. *)
let label_depth (n : var) : int = Int32.to_int n.it

(* Resolve block_type to (n_params, n_results).
   ValBlockType is resolved directly; VarBlockType needs the type section
   and is handled via the optional type_section callback. *)
let block_arity ~type_section (bt : block_type) : (int * int) option =
  match bt with
  | ValBlockType None -> Some (0, 0)
  | ValBlockType (Some _) -> Some (0, 1)
  | VarBlockType v ->
    (match type_section with
     | Some ts ->
       let Wasm_exts.Types.FuncType (ps, rs) = ts v.it in
       Some (List.length ps, List.length rs)
     | None -> None)

(* Process a single instruction, returning updated LRU or None for terminators *)
let rec step ~func_type ~type_section ?on_call lru (instr : instr) : t option =

  let open Wasm_exts.Values in
  (* Disambiguate const_val constructors from Values.I32/I64 *)
  let i32 n : const_val = I32 n in
  let i64 n : const_val = I64 n in
  let from_local n : const_val = FromLocal n in
  let bool32 b = i32 (if b then 1l else 0l) in
  let to_const_val : value -> const_val option = function
    | I32 n -> Some (i32 n) | I64 n -> Some (i64 n) | _ -> None in
  (* Save pending refinement, then clear it. Compare will set a new one;
     BrIf will use the saved one; all other instructions get None. *)
  let pending_refinement = lru.refinement in
  let lru = { lru with refinement = None } in
  match instr.it with
  (* Constants: push onto stack *)
  | Const {it; _} ->
    let lru = shift_and_evict 1 lru in (* existing entries go deeper *)
    (match to_const_val it with
     | Some cv -> Some (insert cv lru)
     | None -> Some lru) (* float: don't track *)

  (* Drop: pop one *)
  | Drop ->
    Some (shift_and_evict (-1) lru)

  (* Nop: no change *)
  | Nop -> Some lru

  (* Binary ops: pop 2, push 1 = net -1 *)
  | Binary (I32 op) ->
    let result =
      match lookup lru 0, lookup lru 1 with
      | Some a, Some b ->
        I32Op.(match op with
         | Add -> try_binop_i32 Int32.add a b
         | Sub -> try_binop_i32 Int32.sub a b
         | Mul -> try_binop_i32 Int32.mul a b
         | _ -> None)
      | _ -> None
    in
    let lru = shift_and_evict (-1) lru in
    (match result with
     | Some v -> Some (insert v lru)
     | None -> Some lru)

  | Binary (I64 op) ->
    let result =
      match lookup lru 0, lookup lru 1 with
      | Some a, Some b ->
        I64Op.(match op with
         | Add -> try_binop_i64 Int64.add a b
         | Sub -> try_binop_i64 Int64.sub a b
         | Mul -> try_binop_i64 Int64.mul a b
         | _ -> None)
      | _ -> None
    in
    let lru = shift_and_evict (-1) lru in
    (match result with
     | Some v -> Some (insert v lru)
     | None -> Some lru)

  | Binary _ ->
    (* Float binary: pop 2, push 1, don't track *)
    Some (shift_and_evict (-1) lru)

  (* Unary ops: pop 1, push 1 = net 0; propagate for known i32 ops *)
  | Unary (I32 op) ->
    let result = match lookup lru 0 with
      | Some (I32 n) -> I32Op.(match op with
         | Clz -> Some (i32 (Wasm.I32.clz n))
         | Ctz -> Some (i32 (Wasm.I32.ctz n))
         | Popcnt -> Some (i32 (Wasm.I32.popcnt n))
         | _ -> None)
      | _ -> None in
    evict_tos lru |> maybe_insert result
  | Unary (I64 op) ->
    let result = match lookup lru 0 with
      | Some (I64 n) -> I64Op.(match op with
         | Clz -> Some (i64 (Wasm.I64.clz n))
         | Ctz -> Some (i64 (Wasm.I64.ctz n))
         | Popcnt -> Some (i64 (Wasm.I64.popcnt n))
         | _ -> None)
      | _ -> None in
    evict_tos lru |> maybe_insert result
  | Unary _ ->
    Some (evict_tos lru)

  (* Test: pop 1, push 1 i32 (boolean result) *)
  | Test (I32 I32Op.Eqz) ->
    let result = match lookup lru 0 with
      | Some (I32 0l) -> Some (i32 1l)
      | Some (I32 _) -> Some (i32 0l)
      | _ -> None in
    evict_tos lru |> maybe_insert result
  | Test (I64 _) ->
    let result = match lookup lru 0 with
      | Some (I64 0L) -> Some (i32 1l)
      | Some (I64 _) -> Some (i32 0l)
      | _ -> None in
    evict_tos lru |> maybe_insert result
  | Test _ ->
    let lru = evict_tos lru in
    Some lru

  (* Compare: pop 2, push 1 i32 (boolean result) *)
  | Compare (I32 op) ->
    let top = lookup lru 0 in
    let snd = lookup lru 1 in
    let result, refine = match top, snd with
      | Some (I32 a), Some (I32 b) ->
        Some I32Op.(match op with
         | Eq  -> bool32 (a = b)
         | Ne  -> bool32 (a <> b)
         | LtS -> bool32 (a < b)
         | LtU -> bool32 (Int32.unsigned_compare a b < 0)
         | GtS -> bool32 (a > b)
         | GtU -> bool32 (Int32.unsigned_compare a b > 0)
         | LeS -> bool32 (a <= b)
         | LeU -> bool32 (Int32.unsigned_compare a b <= 0)
         | GeS -> bool32 (a >= b)
         | GeU -> bool32 (Int32.unsigned_compare a b >= 0)),
        None
      (* FromLocal n vs FromLocal n (same local): Eq is always true, Ne always false *)
      | Some (FromLocal n), Some (FromLocal m) when n = m ->
        I32Op.(match op with
         | Eq -> Some (i32 1l), None
         | Ne -> Some (i32 0l), None
         | _ -> None, None)
      (* FromLocal n vs known constant k (either order): set refinement for Eq/Ne *)
      | Some (FromLocal n), Some ((I32 _ | I64 _) as k)
      | Some ((I32 _ | I64 _) as k), Some (FromLocal n) ->
        I32Op.(match op with
         | Eq -> None, Some { local_idx = n; value = k; sense = true }
         | Ne -> None, Some { local_idx = n; value = k; sense = false }
         | _ -> None, None)
      | _ -> None, None in
    let lru = shift_and_evict (-1) { lru with refinement = refine } in
    lru |> maybe_insert result

  | Compare (I64 op) ->
    let top = lookup lru 0 in
    let snd = lookup lru 1 in
    let result, refine = match top, snd with
      | Some (I64 a), Some (I64 b) ->
        Some I64Op.(match op with
         | Eq  -> bool32 (a = b)
         | Ne  -> bool32 (a <> b)
         | LtS -> bool32 (a < b)
         | LtU -> bool32 (Int64.unsigned_compare a b < 0)
         | GtS -> bool32 (a > b)
         | GtU -> bool32 (Int64.unsigned_compare a b > 0)
         | LeS -> bool32 (a <= b)
         | LeU -> bool32 (Int64.unsigned_compare a b <= 0)
         | GeS -> bool32 (a >= b)
         | GeU -> bool32 (Int64.unsigned_compare a b >= 0)),
        None
      (* FromLocal n vs FromLocal n (same local): Eq is always true, Ne always false *)
      | Some (FromLocal n), Some (FromLocal m) when n = m ->
        I64Op.(match op with
         | Eq -> Some (i32 1l), None
         | Ne -> Some (i32 0l), None
         | _ -> None, None)
      (* FromLocal n vs known constant k (either order): set refinement for Eq/Ne *)
      | Some (FromLocal n), Some ((I32 _ | I64 _) as k)
      | Some ((I32 _ | I64 _) as k), Some (FromLocal n) ->
        I64Op.(match op with
         | Eq -> None, Some { local_idx = n; value = k; sense = true }
         | Ne -> None, Some { local_idx = n; value = k; sense = false }
         | _ -> None, None)
      | _ -> None, None in
    let lru = shift_and_evict (-1) { lru with refinement = refine } in
    lru |> maybe_insert result

  | Compare _ ->
    (* Float compare: pop 2, push 1, don't fold *)
    Some (shift_and_evict (-1) lru)

  (* Select: pop condition + two values, push selected = net -2 *)
  | Select ->
    let result = match lookup lru 0 with
      | Some (I32 0l | I64 0L) -> lookup lru 1                  (* false → val2 *)
      | Some _ -> lookup lru 2                                  (* true  → val1 *)
      | None -> match lookup lru 1, lookup lru 2 with
        | Some a, Some b when a = b -> Some a                   (* both equal *)
        | _ -> None
    in
    let lru = shift_and_evict (-2) lru in
    (match result with
     | Some v -> Some (insert v lru)
     | None -> Some lru)

  (* Local get: push 1; if local is known, propagate; otherwise track origin *)
  | LocalGet x ->
    let lru = shift_and_evict 1 lru in
    (match LocalMap.find_opt x.it lru.locals with
     | Some cv -> Some (insert cv lru)
     | None -> Some (insert (from_local x.it) lru))

  (* Local set: pop 1; if TOS is known, record in locals; evict stale FromLocal *)
  | LocalSet x ->
    let lru = evict_from_local x.it lru in
    let locals = match lookup lru 0 with
      | Some cv -> LocalMap.add x.it cv lru.locals
      | None -> LocalMap.remove x.it lru.locals in
    Some (shift_and_evict (-1) { lru with locals })

  (* Local tee: like set but value stays on stack; evict stale FromLocal *)
  | LocalTee x ->
    let lru = evict_from_local x.it lru in
    let locals = match lookup lru 0 with
      | Some cv -> LocalMap.add x.it cv lru.locals
      | None -> LocalMap.remove x.it lru.locals in
    Some { lru with locals }

  (* Global get/set *)
  | GlobalGet _ -> Some (shift_and_evict 1 lru)
  | GlobalSet _ -> Some (shift_and_evict (-1) lru)

  (* Memory ops *)
  | Load _ -> (* pop addr, push value = net 0, unknown *)
    Some (evict_tos lru)
  | Store _ -> (* pop addr + value = net -2 *) Some (shift_and_evict (-2) lru)
  | MemorySize -> Some (shift_and_evict 1 lru)
  | MemoryGrow -> (* pop 1, push 1 = net 0, unknown *)
    Some (evict_tos lru)

  (* Call: consume n_params, produce n_results (all unknown) *)
  | Call x ->
    let (n_params, n_results) = func_type x.it in
    (* net stack delta: results - params *)
    let lru = shift_and_evict (n_results - n_params) lru in
    Some lru

  | CallIndirect (type_idx, _) ->
    let (n_params, n_results) = func_type type_idx.it in
    (* also consumes the table index from the stack: +1 param *)
    let lru = shift_and_evict (n_results - n_params - 1) lru in
    Some lru

  (* Block: body's branches plus fall-through all feed a shared
     branch_states list; the join is fold-intersect. Depth-decrement
     for n>0 is handled by run_under_leave_handler. *)
  | Block (bt, body) ->
    (match block_arity ~type_section bt with
     | Some (n_params, _n_results) ->
       let lru = shift_and_evict (-n_params) lru in
       let branch_states = ref [] in
       run_under_leave_handler
         ~on_zero:(fun lru -> branch_states := lru :: !branch_states)
         ~normalise:true
         (fun () -> process_block_inner ~func_type ~type_section ?on_call lru body);
       join_branch_states !branch_states
     | None -> None)

  (* Loop: the Wasm label is at loop entry, so depth-0 branches are
     back-edges we don't track (no fixed-point). The body is still
     processed for `on_call` side effects; entry and exit states
     are both fully flushed, so depth shifts are vacuous. *)
  | Loop (bt, body) ->
    (match block_arity ~type_section bt with
     | Some _ ->
       let lru = { lru with entries = []; locals = LocalMap.empty } in
       run_under_leave_handler
         ~on_zero:(fun _ -> ())         (* swallow back-edges *)
         ~normalise:false               (* fall-through state is discarded anyway *)
         (fun () -> process_block_inner ~func_type ~type_section ?on_call lru body);
       Some lru
     | None -> None)

  (* If: both legs share branch_states and the same handler.
     Structurally identical to Block once fall-through is normalised
     into an effect — the two legs just become two invocations of the
     same body-under-handler runner. *)
  | If (bt, then_body, else_body) ->
    (match block_arity ~type_section bt with
     | Some (n_params, _n_results) ->
       let lru = shift_and_evict (-1) lru in           (* pop condition *)
       let lru = shift_and_evict (-n_params) lru in   (* shift for block params *)
       let branch_states = ref [] in
       let on_zero lru = branch_states := lru :: !branch_states in
       let run body =
         run_under_leave_handler ~on_zero ~normalise:true
           (fun () -> process_block_inner ~func_type ~type_section ?on_call lru body) in
       run then_body;
       run else_body;
       join_branch_states !branch_states
     | None -> None)

  | BrIf n ->
    (* Pop condition, emit the branch-taken state as May_leave, continue on fall-through.
       Fall-through means condition was false (0). Apply sense=false refinement. *)
    let lru_fall = match pending_refinement with
      | Some { local_idx; value; sense = false } ->
        { lru with locals = LocalMap.add local_idx value lru.locals }
      | _ -> lru in
    let lru_taken = shift_and_evict (-1) lru in
    let lru_fall  = shift_and_evict (-1) lru_fall in
    May_leave (label_depth n, lru_taken) |> Effect.perform;
    Some lru_fall

  | Br n ->
    May_leave (label_depth n, lru) |> Effect.perform;
    None

  | BrTable _ -> None                    (* not yet modelled *)
  | Return | Unreachable -> None

  (* Bulk memory *)
  | MemoryFill | MemoryCopy | MemoryInit _ -> Some (shift_and_evict (-3) lru)

  (* Convert: pop 1, push 1 = net 0 *)
  | Convert cvt ->
    let result = match cvt, lookup lru 0 with
      | I32 I32Op.WrapI64, Some (I64 n) -> Some (i32 (Wasm.I32_convert.wrap_i64 n))
      | I64 I64Op.ExtendSI32, Some (I32 n) -> Some (i64 (Wasm.I64_convert.extend_i32_s n))
      | I64 I64Op.ExtendUI32, Some (I32 n) -> Some (i64 (Wasm.I64_convert.extend_i32_u n))
      | _ -> None in
    evict_tos lru |> maybe_insert result

  (* Anything else: bail *)
  | _ -> None

and process_block_inner ~func_type ~type_section ?on_call lru instrs =

  let rec go idx lru = function
    | [] -> Some lru
    | instr :: rest ->
      (* Fire callback before processing call instructions *)
      (match on_call, instr.it with
       | Some cb, Call x ->
         let (n_params, n_results) = func_type x.it in
         cb lru idx n_params n_results instr
       | Some cb, CallIndirect (type_idx, _) ->
         let (n_params, n_results) = func_type type_idx.it in
         cb lru idx (n_params + 1) n_results instr (* +1 for table index *)
       | _ -> ());
      match step ~func_type ~type_section ?on_call lru instr with
      | None -> None
      | Some lru' -> go (idx + 1) lru' rest
  in
  go 0 lru instrs

(* Top-level: swallow any May_leave that leaks past the outermost block.
   In valid Wasm, a Br at the top of a function body targets the implicit
   function label, which is a return — the callers of process_block aren't
   interested in that state. *)
let process_block ~func_type ?type_section ?on_call lru instrs =
  let effc : type a. a Effect.t -> ((a, t option) Effect.Deep.continuation -> t option) option = function
    | May_leave (_, _) -> Some (fun k -> Effect.Deep.continue k ())
    | _ -> None in
  Effect.Deep.try_with
    (fun () -> process_block_inner ~func_type ~type_section ?on_call lru instrs)
    ()
    { Effect.Deep.effc = effc }

(* Physical-identity comparison. Sound under the linker's tree-shaped-IR
   invariant (no phrase sharing, no position aliasing). See the .mli. *)
let same_instr (a : instr) (b : instr) : bool = a == b
