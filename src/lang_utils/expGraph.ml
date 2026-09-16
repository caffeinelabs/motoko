module IM = Map.Make(Int)
module IS = Set.Make(Int)

(*
Note [A running example]
~~~~~~~~~~~~~~~~~~~~~~~~

How do we go from

  type A = ?[A]
  type B = [?B]
  t = ([A], B, [B])

to the canonical representation

  t = (1=?[!1], !1, [!1])

in this code? By these transformations:

Step 1: `unfold` will turn this tree into the following graph (depth first,
memoizing graph nodes based on structural equality on the tree)

  0: ("tuple", [1,4,6])
  1: ("vector",[2])
  2: ("opt",   [3])
  3: ("vector",[2])
  4: ("vector",[5])
  5: ("opt",   [4])
  6: ("vector",[4])

Note that this graph is not minimal.

Step 2: `combine` will find the equivalence relations on the nodes of the graph.
To find the coarsest we begin with

   [0 1 2 3 4 5 6]

and now iterate using `equiv_classes`. First distinguishing by constructor

   [0] [1 3 4 6] [2 5]

then by whether the argument list is different (with respect to the current equivalence relation!)

   [0] [1 3 4] [6] [2 5]

Now the iteration stops (could go further with deeper types). Renumbering based
on this equivelence relation gives:

  0: ("tuple", [1,1,2])
  1: ("vector",[3])
  2: ("vector",[1])
  3: ("opt",   [1])

Step 3: `renumber` to get canonical numbers (depth-first traversal)

  0: ("tuple", [1,1,3])
  1: ("vector",[2])
  2: ("opt",   [1])
  3: ("vector",[1])


Step 4: `unfold`

To unfold nicely, the nodes that are referenced more than once are 1, so only
that gets a name. This way we end up with something like

  t = (1=?[!1], !1, [!1])

*)


(* A graph of nodes, nodes labeled by ints, root at node 0 *)
type 'a t = ('a * int list) IM.t

(* Simple counter data structure *)
let start_counting start : (unit -> int) =
  let r = ref start in
  fun () -> let i = !r in r := !r + 1; i

let unfold (type e) node (root : e) : 'a t =
  let module M = Map.Make (struct type t = e let compare = compare end) in

  let seen = ref M.empty in
  let next = start_counting 0 in
  let graph = ref IM.empty in
  let rec go e : int =
    match M.find_opt e !seen with
    | Some i -> i
    | None ->
      let i = next () in
      seen := M.add e i !seen;
      let (k, args) = node e in
      let args' = List.map go args in
      graph := IM.add i (k, args') !graph;
      i
  in
  let i = go root in
  assert (i == 0);
  !graph


(* Maps an index mapping over the graph. If not injective, will combine nodes *)
let rename (lookup : int -> int) graph = graph
    |> IM.to_seq
    |> Seq.map (fun (i, (k, args)) -> (lookup i, (k, List.map lookup args)))
    |> IM.of_seq

(* Given a function on int (given as sequences of points),
   calculates the equivalence classes it represents,
   in the form of a mapping from int to int (plus size)

   Example Input:   Example Output:
     0 ↦ "Ho"         0 ↦ 0
     1 ↦ "Hi"         1 ↦ 1
     3 ↦ "Ho"         3 ↦ 0

*)
let equiv_classes (type b) (graph : (int * b) Seq.t) : (int IM.t * int) =
  let module BM = Map.Make (struct type t = b let compare = compare end) in
  let m = ref BM.empty in
  let next = start_counting 0 in

  let m =
    IM.of_seq (Seq.map (fun (i,y) ->
      match BM.find_opt y !m with
      | Some j -> (i, j)
      | None ->
        let j = next () in
        m := BM.add y j !m;
        (i, j)
    ) graph) in
  let size = next () in
  m, size


(* Children-before-parents order, or None if the graph has a cycle *)
let topo_order (graph : 'a t) : int list option =
  let state = ref IM.empty in (* false = on stack, true = finished *)
  let order = ref [] in
  let cyclic = ref false in
  let rec go i =
    if not !cyclic then
      match IM.find_opt i !state with
      | Some true -> ()
      | Some false -> cyclic := true
      | None ->
        state := IM.add i false !state;
        let (_, args) = IM.find i graph in
        List.iter go args;
        state := IM.add i true !state;
        order := i :: !order
  in
  go 0;
  if !cyclic then None else Some (List.rev !order)

(* On an acyclic graph bisimilarity coincides with structural equality, so the
   coarsest classes can be read off bottom-up in one pass: a node's class is
   determined by its label and its children's classes, which are already final.
   This avoids the fixed-point iteration below, which needs one round per level
   and so is quadratic on deep chains. *)
let combine_acyclic (type a) (graph : a t) (order : int list) : a t =
  let module KM = Map.Make (struct type t = a * int list let compare = compare end) in
  let cls = ref IM.empty in
  let km = ref KM.empty in
  let next = start_counting 0 in
  List.iter (fun i ->
    let (k, args) = IM.find i graph in
    let key = (k, List.map (fun j -> IM.find j !cls) args) in
    let c = match KM.find_opt key !km with
      | Some c -> c
      | None -> let c = next () in km := KM.add key c !km; c in
    cls := IM.add i c !cls
  ) order;
  (* `renumber` starts from node 0, so the root's class must stay 0 *)
  let root = IM.find 0 !cls in
  let lookup i =
    let c = IM.find i !cls in
    if c = root then 0 else if c = 0 then root else c in
  rename lookup graph

(* Finds a minimal graph by finding the smallest index mapping that is consistent *)
(* Equivalently: The coarsest equivalence classes on the nodes *)
let combine_cyclic graph =
  let m : int IM.t ref = ref IM.empty in
  let lookup i = IM.find i !m in
  (* map all nodes to the same initially *)
  IM.iter (fun i _ -> m := IM.add i 0 !m) graph;
  let size = ref 1 in
  let finished = ref false in

  (* Fixed-point iteration *)
  while not !finished do
    (* Update the equivalence classes. By including the previous class,
       this is a refinement *)
    let m', size' = graph
      |> IM.to_seq
      |> Seq.map (fun (i, (k, args)) -> (i, (lookup i, k, List.map lookup args)))
      |> equiv_classes in
    assert (size' >= !size); (* New equivalence class better be finer *)
    finished := size' = !size;
    size := size';
    m := m';
  done;

  assert (lookup 0 = 0);
  rename lookup graph

(* Changes the numbering to be canonical (depth first) *)
let renumber graph =
  let m = ref IM.empty in
  let lookup i = IM.find i !m in
  let next = start_counting 0 in

  let rec go i = match IM.find_opt i !m with
    | None -> (* not seen before *)
      m := IM.add i (next ()) !m;
      let (k, args) = IM.find i graph in
      List.iter go args
    | Some _ -> ()
  in
  go 0;

  assert (lookup 0 = 0);
  rename lookup graph

let combine graph =
  match topo_order graph with
  | Some order -> combine_acyclic graph order
  | None -> combine_cyclic graph

(* Find a canonical graph *)
let canonicalize graph = renumber (combine graph)

(* Folds over the graph *)
let fold
  (of_con : 'a -> 'b list -> 'b)
  (of_def : int -> 'b -> 'b)
  (of_ref : int -> 'b)
  (graph : 'a t) : 'b =

  (* Find which entries are referenced more than once *)
  let tally : int IM.t =
    let tally = ref IM.empty in
    let succ = function | None -> Some 1 | Some i -> Some (i + 1) in
    let bump i = tally := IM.update i succ !tally in
    bump 0;
    IM.iter (fun _ (_, args) -> List.iter bump args) graph;
    !tally
  in

  (* Nodes need an explicit definition if not nullary and referenced
     more than once
   *)
  let needs_def : IS.t =
    IS.of_seq (Seq.filter_map (fun (i, (k, args)) ->
        if args != [] && IM.find i tally > 1 then Some i else None
    ) (IM.to_seq graph))
  in

  (* Now fold the graph using the user-provided combinators *)
  let seen = ref IS.empty in
  let rec go_con i : 'b =
    (* This node is only visited once *)
    let (k, args) = IM.find i graph in
    of_con k (List.map go args)
  and go i : 'b =
    (* This node is only visited once: *)
    if IS.mem i needs_def then
      (* We have seen this before: *)
      if IS.mem i !seen then of_ref i
      (* This is a shared node, first visit: *)
      else (seen := IS.add i !seen; of_def i (go_con i))
    else go_con i
  in
  go 0
