open Type

(*

  Check the non-expansiveness criterion identified first by Viroli and
  adopted by Pierce and Kennedy [1] to ensure termination of sub-typing
  in *nominal* type systems with generics and subtyping.

  Given a set of mutually recursive type definitions, construct a graph
  whose vertices are the formal parameters (identified by position),
  `C#i`, and two sorts of labeled edges:

  * an occurrence of parameter `C#i` as immediate `j`-th argument to
    type `D<..,C#i,..>`, adds a (non-expansive) 0-labeled edge `C#i -0->D#j`.

  * an occurrence of parameter `C#i` as a proper sub-term of the `j`-th
    argument to type `D<...,T[C#i],..>` adds an (expansive) 1-labeled
    edge `C#i -1-> D#j`.

  The graph is expansive iff it contains a cycle with at least one
  expansive edge.

  Non-expansiveness (hopefully) ensures that the set of types
  encountered during sub-typing, and added to the visited set, is
  finite. In nominal systems, a visited set is used to reject a
  cyclic sub-type check, but in our co-inductive setting, it is used
  to succeed a cyclic check. In either case, it is used to terminate the
  procedure, which is what we care about.

  To detect the existence of a cycle, we construct the plain graph
  obtained by deleting labels (possibly identifying edges), compute
  its strongly connected components (sccs), and then check whether
  there is a 1-weighted edge (in the original graph) connecting any
  two vertices of the *same* component of the sccs.

  [1] Andrew Kennedy Benjamin C. Pierce
  International Workshop on Foundations and Developments of Object-Oriented Languages (FOOL/WOOD),
  January 2007

  https://www.microsoft.com/en-us/research/publication/on-decidability-of-nominal-subtyping-with-variance/

*)

let debug = false (* set to 1 to show graph in error message *)

(* Collecting type constructors *)

module Vertex = struct
  type t = Type.con * int
  let compare (c,i) (d,j) =
  match Cons.compare c d with
  | 0 -> compare i j
  | n -> n
end

module Edge = struct
  type t = Vertex.t * int * Vertex.t
  let compare (c1, w1, d1) (c2, w2, d2) =
  match Vertex.compare c1 c2 with
  | 0 ->
    begin
      match compare w1 w2 with
      | 0 -> Vertex.compare d1 d2
      | n -> n
    end
  | n -> n
end

module VertexSet = Set.Make(Vertex)

module EdgeSet = Set.Make(Edge)

let string_of_vertex (c, i) = Printf.sprintf "(%s,%i)" (Cons.name c) i

let string_of_vertices vs =
  Printf.sprintf "{ %s }" (String.concat "," (List.map string_of_vertex (VertexSet.elements vs)))

let string_of_edge (ci,w,dj) =
  Printf.sprintf ("%s -%i-> %s") (string_of_vertex ci) w (string_of_vertex dj)

let string_of_edges es =
  Printf.sprintf "{ %s }" (String.concat "," (List.map string_of_edge (EdgeSet.elements es)))

let edges_typ cs c (es : EdgeSet.t) t : EdgeSet.t =
  let rec go_typs i exp non es ts =
    List.fold_left (go_typ i exp non) es ts
  and go_typ i exp non es = function
    | Var (s, j) when j >= i  ->
      let ci = (c, j - i) in
      let es1 = VertexSet.fold (fun dj es -> EdgeSet.add (ci, 1, dj) es) exp es in
      let es2 = VertexSet.fold (fun dj es -> EdgeSet.add (ci, 0, dj) es) non es1 in
      es2
    | Var (s, j) ->
      assert (j < i);
      es
    | (Prim _ | Any | Non | Pre ) -> es
    | Con (d, ts) when ConSet.mem d cs ->
      let exp1 = VertexSet.union exp non in
      let _, es = List.fold_left
        (fun (k, es) t ->
          (k + 1,
           go_typ i exp1 (VertexSet.singleton (d, k)) es t))
        (0, es)
        ts
      in
      es
    | Con (_, ts) (* Cons from outer scopes are assumed to be non-expansive *)
    | Tup ts ->
      go_typs i (VertexSet.union exp non) VertexSet.empty es ts
    | (Opt t1 | Mut t1 | Array t1 | Weak t1) ->
      go_typ i (VertexSet.union exp non) VertexSet.empty es t1
    | Async (s, t1, t2) ->
      go_typs i (VertexSet.union exp non) VertexSet.empty es [t1;t2]
    | Func (s, _c, tbs, ts1, ts2) ->
      let i1 = i + List.length tbs in
      let exp1 = VertexSet.union exp non in
      let es1 = go_typs i1 exp1 VertexSet.empty es
        (List.map (fun tb -> tb.bound) tbs)
      in
      let es2 = go_typs i1 exp1 VertexSet.empty es1 ts1
      in
      go_typs i1 exp1 VertexSet.empty es2 ts2
    | (Obj (_, fs, _) | Variant fs) ->
      go_typs i (VertexSet.union exp non) VertexSet.empty es
        (List.map (fun f -> f.typ) fs)
    | Named (n, t1) ->
       go_typ i exp non es t1 (* TBR *)
  in
  go_typ 0 VertexSet.empty VertexSet.empty es t

let edges_con cs c es : EdgeSet.t =
  match Cons.kind c with
  | Def (tbs, t) ->
    (* It's not clear we actually need to consider parameters bounds, since, unlike
       function type parameters, they don't introduce new subgoals during subtyping.
       But let's be conservative and consider them, until we find out that that's undesirable
       and know its safe to ignore them here. *)
    let es1 = List.fold_left (fun es tb ->
      edges_typ cs c es tb.bound) es tbs
    in
    edges_typ cs c es1 t
  | Abs (tbs, t) ->
    assert false

let edges cs = ConSet.fold (edges_con cs) cs EdgeSet.empty

let vertices cs =
  ConSet.fold
    (fun c vs ->
      match Cons.kind c with
      | Def (tbs, t) ->
        let ws = List.mapi (fun i _tb -> (c, i)) tbs in
        List.fold_left (fun vs v -> VertexSet.add v vs) vs ws
      | Abs (tbs, t) ->
        assert false) cs VertexSet.empty

module VertexMap = Map.Make(Vertex)

module Scc = Scc.Make(Vertex)

let string_of_sccs sccs =
  Printf.sprintf "{ %s }" (String.concat ","
   (List.map string_of_vertices sccs))

module Pretty = MakePretty(ElideStamps)

let is_expansive cs =
  (* Collect vertices and labeled edges *)
  let vs = vertices cs in
  let es = edges cs in
  (* Compute the strongly connected components (ignoring edge labels) *)
  let unlabeled_es = EdgeSet.fold
    (fun (ci, w, dj) -> Scc.EdgeSet.add (ci, dj)) es Scc.EdgeSet.empty
  in
  let vss = Scc.scc vs unlabeled_es in

  (* Map each vertex to the number of its component *)
  let component = List.fold_left (fun m (vs, i) ->
    VertexSet.fold (fun v m -> VertexMap.add v i m) vs m)
    VertexMap.empty (List.mapi (fun i vs -> (vs, i)) vss)
  in

  (* The constructor are expansive if some component (cycle) contains
     an edge with non-zero weight *)
  let e_opt = List.find_opt
    (fun (ci, w, dj) ->
      w > 0 && VertexMap.find ci component = VertexMap.find dj component)
    (EdgeSet.elements es)
  in
  match e_opt with
  | None -> None
  | Some ((c,i), _, (d,j)) ->
    (* Construct an error messages with optional debug info *)
    let op, sbs, st = Pretty.strings_of_kind (Cons.kind c) in
    let def = Printf.sprintf "type %s%s %s %s" (Cons.name c) sbs op st in
    let x = match Cons.kind c with Def(tbs, _) | Abs(tbs, _) ->
      (List.nth tbs i).var in
    let dys = match Cons.kind d with Def(tbs, _) | Abs(tbs, _) ->
      Printf.sprintf "%s<%s>" (Cons.name d)
        (String.concat "," (List.mapi (fun k _ ->
          if i = k then "-" ^ x ^"-" else "_") tbs))
    in
    Some (Printf.sprintf
      ":\n  %s\nis expansive, because %s occurs as an indirect argument of recursive type %s.\n(%s would be allowed as an immediate argument, but cannot be part of a larger type expression.)%s"
      def x dys x
      (if debug then
         Printf.sprintf
           "\n  vertices:\n    %s\n  edges:\n    %s\n  components:\n    %s"
           (string_of_vertices vs) (string_of_edges es) (string_of_sccs vss)
       else ""))
