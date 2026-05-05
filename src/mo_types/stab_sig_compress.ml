open Type

module HashMap = Map.Make (String)

(* === Generic stab_sig field traversals ====================================== *)

let map_fields f = function
  | Single fs -> Single (List.map f fs)
  | PrePost (pre, post) ->
    PrePost (List.map (fun (req, fld) -> (req, f fld)) pre,
             List.map f post)
  | Multi { chain; post } ->
    Multi { chain = List.map f chain; post = List.map f post }

let fold_fields f init = function
  | Single fs -> List.fold_left f init fs
  | PrePost (pre, post) ->
    let acc = List.fold_left (fun acc (_, fld) -> f acc fld) init pre in
    List.fold_left f acc post
  | Multi { chain; post } ->
    let acc = List.fold_left f init chain in
    List.fold_left f acc post

let collect_field cs f = ConSet.union cs (cons f.typ)

(* === Substitution ========================================================== *)

(* Like [Type.subst] but additionally rewrites cons stored in [Obj]'s
   type-component list ([tfs]); safe here because every value in [cmap]
   is a closed cons reference. *)
let rec rewrite cmap t =
  match t with
  | Var _ | Prim _ | Any | Non | Pre -> t
  | Con (c, ts) ->
    let c' = try ConEnv.find c cmap with Not_found -> c in
    Con (c', List.map (rewrite cmap) ts)
  | Opt t -> Opt (rewrite cmap t)
  | Mut t -> Mut (rewrite cmap t)
  | Array t -> Array (rewrite cmap t)
  | Weak t -> Weak (rewrite cmap t)
  | Tup ts -> Tup (List.map (rewrite cmap) ts)
  | Async (s, t1, t2) -> Async (s, rewrite cmap t1, rewrite cmap t2)
  | Func (s, c, tbs, ts1, ts2) ->
    Func (s, c,
          List.map (fun b -> { b with bound = rewrite cmap b.bound }) tbs,
          List.map (rewrite cmap) ts1,
          List.map (rewrite cmap) ts2)
  | Obj (s, fs, tfs) ->
    Obj (s,
         List.map (fun f -> { f with typ = rewrite cmap f.typ }) fs,
         List.map (fun f ->
           let c' = try ConEnv.find f.typ cmap with Not_found -> f.typ in
           { f with typ = c' }) tfs)
  | Variant fs ->
    Variant (List.map (fun f -> { f with typ = rewrite cmap f.typ }) fs)
  | Named (n, t) -> Named (n, rewrite cmap t)

let rewrite_field cmap f = { f with typ = rewrite cmap f.typ }

(* === Pass 1: structural dedup ============================================== *)

let rec is_bare_alias = function
  | Con _ -> true
  | Mut t -> is_bare_alias t
  | _ -> false

let pick_rep = function
  | [] -> assert false
  | first :: rest ->
    List.fold_left
      (fun best c -> if Cons.compare c best < 0 then c else best)
      first rest

let dedup sig_ =
  let all_cons = fold_fields collect_field ConSet.empty sig_ in
  let groups =
    ConSet.fold (fun c acc ->
      match Cons.kind c with
      | Def ([], body) when not (is_bare_alias body) ->
        (try
           let h = Typ_hash.typ_hash body in
           let lst = try HashMap.find h acc with Not_found -> [] in
           HashMap.add h (c :: lst) acc
         with _ -> acc)
      | _ -> acc) all_cons HashMap.empty
  in
  let cmap =
    HashMap.fold (fun _ members acc ->
      match members with
      | [] | [_] -> acc
      | _ ->
        let rep = pick_rep members in
        List.fold_left (fun acc c ->
          if Cons.eq c rep then acc
          else ConEnv.add c rep acc) acc members) groups ConEnv.empty
  in
  if ConEnv.is_empty cmap then sig_
  else begin
    ConSet.iter (fun c ->
      if not (ConEnv.mem c cmap) then
        match Cons.kind c with
        | Def (tbs, body) ->
          Cons.unsafe_set_kind c (Def (tbs, rewrite cmap body))
        | Abs _ -> ()) all_cons;
    map_fields (rewrite_field cmap) sig_
  end

(* === Pass 2: anchored-intersection mining ================================== *)

(* Field dictionary: assign a stable int id per (label, mutability,
   structural type) triple. *)

let field_key f =
  let mut, t = match f.typ with Mut t -> "v", t | t -> "i", t in
  Printf.sprintf "%s|%s|%s" f.lab mut (Typ_hash.typ_hash t)

type dict = {
  mutable next_id : int;
  ids : (string, int) Hashtbl.t;
  by_id : (int, field) Hashtbl.t;
}

let make_dict () =
  { next_id = 0;
    ids = Hashtbl.create 256;
    by_id = Hashtbl.create 256 }

let intern d f =
  let k = field_key f in
  match Hashtbl.find_opt d.ids k with
  | Some id -> id
  | None ->
    let id = d.next_id in
    Hashtbl.add d.ids k id;
    Hashtbl.add d.by_id id f;
    d.next_id <- id + 1;
    id

let field_of_id d id = Hashtbl.find d.by_id id

(* Sorted int array set ops *)

let arr_inter a b =
  let na = Array.length a and nb = Array.length b in
  let buf = Array.make (min na nb) 0 in
  let k = ref 0 and i = ref 0 and j = ref 0 in
  while !i < na && !j < nb do
    if a.(!i) = b.(!j) then (buf.(!k) <- a.(!i); incr k; incr i; incr j)
    else if a.(!i) < b.(!j) then incr i
    else incr j
  done;
  Array.sub buf 0 !k

let arr_diff a b =
  let na = Array.length a and nb = Array.length b in
  let buf = Array.make na 0 in
  let k = ref 0 and i = ref 0 and j = ref 0 in
  while !i < na do
    if !j >= nb || a.(!i) < b.(!j) then (buf.(!k) <- a.(!i); incr k; incr i)
    else if a.(!i) = b.(!j) then (incr i; incr j)
    else incr j
  done;
  Array.sub buf 0 !k

let arr_key a =
  let buf = Buffer.create (Array.length a * 6) in
  Array.iter (fun x ->
    Buffer.add_string buf (string_of_int x);
    Buffer.add_char buf ',') a;
  Buffer.contents buf

(* A cons is factorable if its kind is [Def([], Obj(Object, fs, []))] —
   a plain object record with no type-component fields — and it is not
   structurally self-recursive (recursive types cannot be intersected
   with [and] because [Type.glb] rejects forward/recursive references). *)

let factorable c =
  match Cons.kind c with
  | Def ([], (Obj (Object, fs, []) as body)) ->
    if ConSet.mem c (cons body) then None
    else
      (try
        let _ = Typ_hash.typ_hash body in
        Some fs
      with _ -> None)
  | _ -> None

type record = {
  cons : con;
  mutable parents : con list;
  mutable delta : int array;
}

(* Cost model: declaring a base of [s] fields used by [r] records saves
   ~ (r-1) * (s-1) - 1 field-equivalents.  Threshold: positive savings. *)
let score size users = (users - 1) * (size - 1) - 1

let mine sig_ =
  let all_cons = fold_fields collect_field ConSet.empty sig_ in
  let dict = make_dict () in
  let records = ConSet.fold (fun c acc ->
    match factorable c with
    | None -> acc
    | Some fs ->
      let ids = Array.of_list (List.map (intern dict) fs) in
      Array.sort compare ids;
      { cons = c; parents = []; delta = ids } :: acc
  ) all_cons [] in

  let bases : con list ref = ref [] in
  let base_idx = ref 0 in

  let mine_step () =
    (* Inverted index over current deltas *)
    let inv : (int, record list) Hashtbl.t = Hashtbl.create 256 in
    List.iter (fun r ->
      Array.iter (fun id ->
        let lst = try Hashtbl.find inv id with Not_found -> [] in
        Hashtbl.replace inv id (r :: lst)
      ) r.delta
    ) records;
    (* Anchored intersections, deduplicated by canonical bitmap *)
    let candidates : (string, int array * record list) Hashtbl.t =
      Hashtbl.create 64 in
    Hashtbl.iter (fun _id users ->
      if List.length users >= 2 then begin
        let inter = match users with
          | r :: rs ->
            List.fold_left (fun acc r -> arr_inter acc r.delta) r.delta rs
          | [] -> assert false
        in
        if Array.length inter >= 2 then
          Hashtbl.replace candidates (arr_key inter) (inter, users)
      end
    ) inv;
    Hashtbl.fold (fun _ (inter, users) acc ->
      let s = score (Array.length inter) (List.length users) in
      if s <= 0 then acc
      else match acc with
        | None -> Some (s, inter, users)
        | Some (s', _, _) when s > s' -> Some (s, inter, users)
        | _ -> acc
    ) candidates None
  in

  let rec loop () =
    match mine_step () with
    | None -> ()
    | Some (_, inter, users) ->
      let fs = Array.to_list inter
        |> List.map (field_of_id dict)
        |> List.sort compare_field in
      let body = Obj (Object, fs, []) in
      let base =
        Cons.fresh (Printf.sprintf "Base__%d" !base_idx) (Def ([], body)) in
      incr base_idx;
      bases := base :: !bases;
      List.iter (fun u ->
        u.parents <- base :: u.parents;
        u.delta <- arr_diff u.delta inter
      ) users;
      loop ()
  in
  loop ();

  let factor : (con, con list * field list) Hashtbl.t = Hashtbl.create 64 in
  List.iter (fun r ->
    if r.parents <> [] then begin
      let delta_fs = Array.to_list r.delta
        |> List.map (field_of_id dict)
        |> List.sort compare_field in
      Hashtbl.add factor r.cons (List.rev r.parents, delta_fs)
    end
  ) records;
  let extras =
    List.fold_left (fun s c -> ConSet.add c s) ConSet.empty !bases in
  (extras, factor)

(* === Top-level ============================================================= *)

let string_of sig_ =
  Cons.session ~scope:"stab_sig_compress" (fun () ->
    let sig_ = dedup sig_ in
    let extras, factor = mine sig_ in
    let lookup c = Hashtbl.find_opt factor c in
    string_of_stab_sig_factored ~extras ~factor:lookup sig_)
