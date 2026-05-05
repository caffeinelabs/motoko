open Type

module HashMap = Map.Make (String)

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

let _ = map_fields  (* may become useful for future passes *)

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
   are excluded from grouping: a substitution that collapses them onto a
   different cons would produce trivial self-cycles like [type X = X]. *)
let rec is_bare_alias = function
  | Con _ -> true
  | Mut t -> is_bare_alias t
  | _ -> false

(* Build a cons-to-cons substitution that folds every group of
   structurally-equivalent zero-arity, parameter-less constructors onto a
   single canonical representative. The Motoko type graph is left
   untouched: callers are expected to apply the substitution only at
   serialization time (see [Type.string_of_stab_sig_with_subst]). *)
let build_subst sig_ =
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
  fun c -> ConEnv.find_opt c cmap

(* Some small unit tests *)
[@@@warning "-32"]

let%test "build_subst-collapses-equal-variants" =
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
    let subst = build_subst sig_ in
    (* Exactly one of the two cons should be redirected to the other. *)
    match subst event_a, subst event_b with
    | Some r, None -> Cons.eq r event_b
    | None, Some r -> Cons.eq r event_a
    | _ -> false)

let%test "build_subst-preserves-bare-aliases" =
  Cons.session ~scope:"dedup-bare-alias-test" (fun () ->
    let prim_nat = Cons.fresh "PrimNat" (Def ([], nat)) in
    let alias_nat = Cons.fresh "AliasNat" (Def ([], Con (prim_nat, []))) in
    let sig_ = Single [
      { lab = "v"; typ = Con (alias_nat, []); src = empty_src };
    ] in
    let subst = build_subst sig_ in
    (* Bare aliases must not collapse: that would yield [type X = X]. *)
    subst alias_nat = None && subst prim_nat = None)
