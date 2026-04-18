(* Abstract interpreter for tracking constant integral values on the Wasm operand stack.
   See constTrack.mli for the interface description. *)

open Wasm.Source
open Wasm_exts.Ast

type const_val =
  | I32 of Int32.t
  | I64 of Int64.t

(* LRU entry: stack depth and the constant value *)
type entry = { depth : int; value : const_val }

(* Pure LRU: a list of entries ordered by recency (most recent first),
   with a capacity cap. *)
type t = {
  capacity : int;
  entries : entry list;
  (* High water mark: net stack depth consumed by the last instruction.
     Used to shift depths when processing sequences. *)
}

let empty capacity = { capacity; entries = [] }

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
  let buf = Buffer.create 64 in
  Buffer.add_string buf "LRU[";
  List.iter (fun (d, v) ->
    Printf.bprintf buf " %d:" d;
    (match v with
     | I32 n -> Printf.bprintf buf "i32(%ld)" n
     | I64 n -> Printf.bprintf buf "i64(%Ld)" n);
  ) (entries lru);
  Buffer.add_string buf " ]";
  Buffer.contents buf

(* Intersect two LRUs: keep only entries present in both at the same depth with the same value *)
let intersect lru1 lru2 =
  let entries =
    List.filter (fun e1 ->
      List.exists (fun e2 -> e2.depth = e1.depth && e2.value = e1.value) lru2.entries
    ) lru1.entries
  in
  { lru1 with entries }

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
let is_zero (e : entry) =
  match e.value with I32 0l | I64 0L -> true | _ -> false

let rec step ~func_type ~type_section ?on_call lru (instr : instr) : t option =

  let open Wasm_exts.Values in
  match instr.it with
  (* Constants: push onto stack *)
  | Const { it = I32 c; _ } ->
    let lru = shift_and_evict 1 lru in (* existing entries go deeper *)
    Some (insert (I32 c) lru)
  | Const { it = I64 c; _ } ->
    let lru = shift_and_evict 1 lru in
    Some (insert (I64 c) lru)
  | Const _ ->
    (* Float constants: just shift, don't track *)
    Some (shift_and_evict 1 lru)

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
        (match op with
         | I32Op.Add -> try_binop_i32 Int32.add a b
         | I32Op.Sub -> try_binop_i32 Int32.sub a b
         | I32Op.Mul -> try_binop_i32 Int32.mul a b
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
        (match op with
         | I64Op.Add -> try_binop_i64 Int64.add a b
         | I64Op.Sub -> try_binop_i64 Int64.sub a b
         | I64Op.Mul -> try_binop_i64 Int64.mul a b
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

  (* Unary ops: pop 1, push 1 = net 0, but result unknown *)
  | Unary _ ->
    (* Could propagate for known ops, but skip for now *)
    let lru = { lru with entries = List.filter (fun e -> e.depth <> 0) lru.entries } in
    Some lru

  (* Select: pop 3, push 1 = net -2, result unknown *)
  | Select ->
    Some (shift_and_evict (-2) lru)

  (* Local get: push 1, value unknown for now (Phase 2) *)
  | LocalGet _ ->
    Some (shift_and_evict 1 lru)

  (* Local set: pop 1 *)
  | LocalSet _ ->
    Some (shift_and_evict (-1) lru)

  (* Local tee: pop 1, push 1 = net 0, but we lose track *)
  | LocalTee _ -> Some lru

  (* Global get/set *)
  | GlobalGet _ -> Some (shift_and_evict 1 lru)
  | GlobalSet _ -> Some (shift_and_evict (-1) lru)

  (* Memory ops *)
  | Load _ -> (* pop addr, push value = net 0, unknown *) Some lru
  | Store _ -> (* pop addr + value = net -2 *) Some (shift_and_evict (-2) lru)
  | MemorySize -> Some (shift_and_evict 1 lru)
  | MemoryGrow -> (* pop 1, push 1 = net 0, unknown *) Some lru

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

  (* Block: process body, result type determines net stack effect *)
  | Block (bt, body) ->
    (match block_arity ~type_section bt with
     | Some (n_params, n_results) ->
       let inner_lru = shift_and_evict (-n_params) lru in
       (match process_block_inner ~func_type ~type_section ?on_call inner_lru body with
        | Some lru' -> Some (shift_and_evict (n_results) { lru' with capacity = lru.capacity })
        | None ->
          (* Body hit a branch/terminator; assume worst case: all unknown *)
          Some (shift_and_evict (n_results - n_params) { lru with entries = [] }))
     | None -> None)

  (* Loop: body may repeat, conservatively flush LRU for the body *)
  | Loop (bt, _body) ->
    (match block_arity ~type_section bt with
     | Some (n_params, n_results) ->
       Some (shift_and_evict (n_results - n_params) { lru with entries = [] })
     | None -> None)

  (* If: pop condition, fork into then/else, intersect at join *)
  | If (bt, then_body, else_body) ->
    (match block_arity ~type_section bt with
     | Some (n_params, n_results) ->
       (* pop condition *)
       let lru_cond = shift_and_evict (-1) lru in
       let inner_lru = shift_and_evict (-n_params) lru_cond in
       let then_lru = process_block_inner ~func_type ~type_section ?on_call inner_lru then_body in
       let else_lru = process_block_inner ~func_type ~type_section ?on_call inner_lru else_body in
       let joined = match then_lru, else_lru with
         | Some t, Some e -> intersect t e
         | Some t, None -> { t with entries = [] } (* else hit terminator *)
         | None, Some e -> { e with entries = [] }
         | None, None -> { inner_lru with entries = [] }
       in
       Some (shift_and_evict n_results joined)
     | None -> None)

  | Br _ | BrIf _ | BrTable _ -> None
  | Return -> None
  | Unreachable -> None

  (* Bulk memory *)
  | MemoryFill -> Some (shift_and_evict (-3) lru)
  | MemoryCopy -> Some (shift_and_evict (-3) lru)
  | MemoryInit _ -> Some (shift_and_evict (-3) lru)

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

let process_block ~func_type ?type_section ?on_call lru instrs =
  process_block_inner ~func_type ~type_section ?on_call lru instrs
