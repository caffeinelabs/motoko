open Type

module HashMap = Map.Make (String)

(* Substitute Con references in [t] using [cmap : con -> con].
   Like [Type.subst] but additionally rewrites the cons stored in
   [Obj]'s type-component list ([tfs]); safe here because every value
   in [cmap] is a closed cons reference. *)
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

(* Pick a deterministic representative for a non-empty group: the cons
   with the smallest [Cons.compare]. *)
let pick_rep = function
  | [] -> assert false
  | first :: rest ->
    List.fold_left
      (fun best c -> if Cons.compare c best < 0 then c else best)
      first rest

(* Bodies that are bare aliases (a chain of [Mut] wrapping a single [Con])
   are excluded from grouping: substituting them would produce trivial
   self-cycles like [type X = X], which downstream type-checking cannot
   handle. *)
let rec is_bare_alias = function
  | Con _ -> true
  | Mut t -> is_bare_alias t
  | _ -> false

let dedup sig_ =
  let all_cons = fold_fields collect_field ConSet.empty sig_ in
  (* Group dedupable cons (Def with no binders) by structural hash of
     their body. Skip cons whose body [Typ_hash.typ_hash] cannot
     handle (e.g. references abstract types or async) and cons whose
     body is a bare type alias. *)
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
  (* Build c -> rep mapping for groups of size > 1. *)
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
    (* Mutate every reachable Def cons (except the deduped non-reps,
       which become unreachable after substitution) so its body uses
       canonical reps. The rewritten body is structurally equivalent
       to the original, so downstream typing/codegen is unaffected. *)
    ConSet.iter (fun c ->
      if not (ConEnv.mem c cmap) then
        match Cons.kind c with
        | Def (tbs, body) ->
          Cons.unsafe_set_kind c (Def (tbs, rewrite cmap body))
        | Abs _ -> ()) all_cons;
    map_fields (rewrite_field cmap) sig_
  end

(* Some small unit tests *)
[@@@warning "-32"]

let%test "dedup-collapses-equal-variants" =
  Cons.session ~scope:"dedup-test" (fun () ->
    let body = Variant [
      { lab = "added"; typ = nat; src = empty_src };
      { lab = "removed"; typ = nat; src = empty_src };
    ] in
    let event_a = Cons.fresh "EventA" (Def ([], body)) in
    let event_b = Cons.fresh "EventB" (Def ([], body)) in
    let sig_ = Single [
      { lab = "events_a"; typ = Con (event_a, []); src = empty_src };
      { lab = "events_b"; typ = Con (event_b, []); src = empty_src };
    ] in
    let sig' = dedup sig_ in
    let cs = match sig' with
      | Single fs -> List.fold_left (fun s f -> ConSet.union s (cons f.typ)) ConSet.empty fs
      | _ -> assert false
    in
    (* Both fields now reference the same canonical cons. *)
    ConSet.cardinal cs = 1)

let%test "dedup-preserves-bare-aliases" =
  Cons.session ~scope:"dedup-bare-alias-test" (fun () ->
    let prim_nat = Cons.fresh "PrimNat" (Def ([], nat)) in
    let alias_nat = Cons.fresh "AliasNat" (Def ([], Con (prim_nat, []))) in
    let sig_ = Single [
      { lab = "v"; typ = Con (alias_nat, []); src = empty_src };
    ] in
    let sig' = dedup sig_ in
    (* Bare aliases must not collapse: that would yield [type X = X]. *)
    match sig' with
    | Single [{ typ = Con (c, _); _ }] -> Cons.eq c alias_nat
    | _ -> false)

