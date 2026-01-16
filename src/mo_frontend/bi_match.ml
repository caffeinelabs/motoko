open Mo_types
open Type

(* TODO: consider turning off show_stamps (but then do it elsewhere too)
open MakePretty(struct let show_stamps = false end)
*)

(** Turn on/off debug prints *)
let debug = false

let pp_rel ppf (t1, rel, t2) =
  Format.fprintf ppf "@[<hv 2>%a  %s @ %a@]"
    pp_typ t1
    rel
    pp_typ t2

let pp_constraint ppf (lb, c, ub) =
  Format.fprintf ppf "@[<hv 2>%a  <: @ @[<hv 2>%s  <: @ %a@]@]"
    pp_typ lb
    (Cons.name c)
    pp_typ ub

let display_constraint = Lib.Format.display pp_constraint
let display_constraints f = List.iter (display_constraint f)
let display_rel = Lib.Format.display pp_rel
let display_rels f = List.iter (display_rel f)
let display_typ = Lib.Format.display pp_typ

(* Bi-Matching *)

type reason =
  { actual : typ; expected : typ; at : Source.region }

exception Bimatch of {
  message : string;
  hint : string option;
  reason : reason option;
}

let bimatch ?(hint=None) ?(reason=None) message =
  Bimatch { message; hint; reason }

let error ?(hint=None) ?(reason=None) message =
  raise (bimatch ~hint ~reason message)

(* add a dummy name to recognize the return type *)
let name_ret_typ typ = Named ("@ret", typ)

type var_info = {
  (* Type.Con for this type variable *)
  t : typ;
  (* Input type parameter; used for validation *)
  bind : bind;
}

type ctx = {
  (* Set of type variables being solved *)
  var_set : ConSet.t;
  (* Type variables info *)
  var_env : var_info ConEnv.t;
  (* Lower and upper bounds for type variables *)
  bounds : typ ConEnv.t * typ ConEnv.t;
  (* Variances for type variables *)
  variances : Variance.t ConEnv.t;
  (* Optional return type *)
  ret_typ : typ option;
  (* Initial list of all input type parameters to solve *)
  all_vars : con list;
  (* Current combined solution of all previous rounds *)
  current_env : typ ConEnv.t;
  (* Optional subtyping constraints to verify the solution in the last round *)
  to_verify : typ list * typ list;
}

let empty_ctx env = {
  var_set = ConSet.empty;
  var_env = ConEnv.empty;
  bounds = (ConEnv.empty, ConEnv.empty);
  variances = ConEnv.empty;
  ret_typ = None;
  all_vars = [];
  current_env = env;
  to_verify = ([], []);
}

let is_ctx_empty ctx = ConSet.is_empty ctx.var_set

let verify_ctx ctx =
  assert (ConSet.equal ctx.var_set (ConEnv.dom ctx.var_env));
  assert (ConSet.equal ctx.var_set (ConEnv.dom ctx.variances));
  assert (ConSet.equal ctx.var_set (ConEnv.dom (fst ctx.bounds)));
  assert (ConSet.equal ctx.var_set (ConEnv.dom (snd ctx.bounds)));
  assert (ConSet.subset ctx.var_set (ConSet.of_list ctx.all_vars))

let string_of_bounds (l, u) =
  String.concat ", " (List.map (fun (c, t) -> Printf.sprintf "%s <: %s <: %s" (string_of_typ t) (Cons.name c) (string_of_typ (ConEnv.find c u))) (ConEnv.bindings l))

(** Functions used only for debugging *)
module Debug = struct
  let print_solve ctx (ts1, ts2) must_solve =
    print_endline "solve ctx";
    print_endline (Printf.sprintf "var_set: %s" (String.concat ", " (List.map Cons.name (ConSet.elements ctx.var_set))));
    print_endline (Printf.sprintf "bounds: %s" (string_of_bounds ctx.bounds));
    print_endline (Printf.sprintf "variances: %s" (String.concat ", " (List.map (fun (c, t) -> Printf.sprintf "%s: %s" (Cons.name c) (Variance.string_of t)) (ConEnv.bindings ctx.variances))));
    print_endline (Printf.sprintf "subs: %s" (String.concat ", " (List.map (fun (t1, t2) -> Printf.sprintf "%s <: %s" (string_of_typ t1) (string_of_typ t2)) (List.combine ts1 ts2))));
    print_endline (Printf.sprintf "must_solve : %s" (String.concat ", " (List.map string_of_typ must_solve)));
    verify_ctx ctx

  let print_variables_to_defer used to_defer to_solve =
    print_endline (Printf.sprintf "used : %s" (String.concat ", " (List.map Cons.name (ConSet.elements used))));
    print_endline (Printf.sprintf "to_defer : %s" (String.concat ", " (List.map Cons.name (ConSet.elements to_defer))));
    print_endline (Printf.sprintf "to_solve : %s" (String.concat ", " (List.map Cons.name (ConSet.elements to_solve))))

  let print_solved_bounds l u =
    print_endline (Printf.sprintf "bi_match_typs : %s" (string_of_bounds (l, u)))

  let print_partial_solution env unsolved =
    print_endline (Printf.sprintf "env : %s" (String.concat ", " (List.map (fun (c, t) -> Printf.sprintf "%s := %s" (Cons.name c) (string_of_typ t)) (ConEnv.bindings env))));
    print_endline (Printf.sprintf "unsolved : %s" (String.concat ", " (List.map Cons.name (ConSet.elements unsolved))));
    print_endline ""

  let print_update_bound c current t updated =
    print_endline (Printf.sprintf "update_bound %s: current %s with %s to %s"
      (Cons.name c)
      (string_of_typ current)
      (string_of_typ t)
      (string_of_typ updated))
end

module SS = Set.Make (OrdPair)

(* Types that are denotable (ranged over) by type variables *)
let denotable t =
  let t' = normalize t in
  not (is_mut t')

let bound c = match Cons.kind c with
  | Abs ([], t) -> t
  | _ -> assert false

let as_con_var t = match as_con t with
  | c, [] -> c
  | _ -> assert false

let is_unsolved_var ctx t =
  match t with
  | Con (c, []) -> ConSet.mem c ctx.var_set
  | _ -> false

(** Check partial instantiation [env] satisfies bounds and all the pairwise sub-typing relations in [ts1, ts2];
    used to sanity check inferred instantiations *)
let verify_inst ~ctx ~remaining (ts1, ts2) =
  let env = remaining.current_env in
  ConEnv.for_all (fun c { t; bind } ->
    (* NB: bounds are closed, no need to substitute *)
    is_unsolved_var remaining t || sub (ConEnv.find c env) bind.bound) ctx.var_env &&
  List.for_all2 (fun t1 t2 -> sub (subst env t1) (subst env t2)) ts1 ts2

let mentions typ cons = not (ConSet.disjoint (Type.cons typ) cons)

let fail_open_bound c bd =
  let c = Cons.name c in
  error (Format.asprintf
    "type parameter `%s` has a bound %a\ninvolving another type parameter. Please provide an explicit instantiation."
    c (Lib.Format.display pp_typ) bd)

module ErrorUnderconstrained : sig
  type t
  val empty : unit -> t
  val add : t -> typ -> con -> typ -> unit
  val to_string : t -> string
end = struct
  type t = (typ * con * typ) list ref
  let empty () = ref []
  let add t lb c ub = t := (lb, c, ub) :: !t

  let to_string t =
    let parts = List.rev !t in
    if parts = [] then "" else
    let s = if List.length parts > 1 then "s" else "" in
    Format.asprintf
      "there is no \"best\" choice for type parameter%s `%s`."
      s
      (String.concat "`, `" (List.map (fun (_, c, _) -> Cons.name c) parts))

end


let impossible_over_constrained lb c ub =
  error (Format.asprintf
    "bug: impossible over-constrained type parameter `%s` with%a\nwhere%a\nPlease report this bug and supply an explicit instantiation instead."
    (Cons.name c)
    display_constraint (lb, c, ub)
    display_rel (lb, "</:", ub))

let choose_under_constrained ctx er lb c ub =
  match ConEnv.find c ctx.variances with
  | Variance.Covariant -> lb
  | Variance.Contravariant -> ub
  | Variance.Bivariant -> lb
  | Variance.Invariant ->
    match normalize lb, normalize ub with
    (* Ignore [Any] when choosing a bound for the solution *)
    (* When the solution is between [t] and [Any], choose [t] when there are no other choices except [Any] *)
    | t, Any when has_no_supertypes t ->
      assert (t <> Non);
      lb
    | Non, t when has_no_subtypes t ->
      assert (t <> Any);
      ub
    (* Error otherwise, but pick an arbitrary bound for error reporting *)
    | t, _ ->
      ErrorUnderconstrained.add er lb c ub;
      if t = Non then ub else lb

let check c (l, u) =
  let lb = ConEnv.find c l in
  let ub = ConEnv.find c u in
  if not (sub lb ub) then
    (* Catch the over-constrained error early *)
    None
  else
    Some (l, u)

let update binop c t ce =
  let current = ConEnv.find c ce in
  let updated = binop ?src_fields:None t current in
  if debug then
    Debug.print_update_bound c current t updated;
  (* Future work: consider an error when joining two unrelated types, e.g. [lub Nat Text = Any], would be a breaking change *)
  ConEnv.add c updated ce

let bi_match_typs ctx =
  let flexible c = ConSet.mem c ctx.var_set in

  let rec bi_match_list_result p rel eq inst any xs1 xs2 ats =
    match xs1, xs2, ats with
    | x1::xs1', x2::xs2', at::ats' ->
      (match p rel eq inst any x1 x2 with
      | Some inst -> bi_match_list_result p rel eq inst any xs1' xs2' ats'
      | None -> Result.Error (inst, (x1, x2, at)))
    | [], [], [] -> Ok inst
    | _, _, _ -> assert false
  in

  let rec bi_match_list p rel eq inst any xs1 xs2 =
    match (xs1, xs2) with
    | x1::xs1, x2::xs2 ->
      (match p rel eq inst any x1 x2 with
      | Some inst -> bi_match_list p rel eq inst any xs1 xs2
      | None -> None)
    | [], [] -> Some inst
    | _, _ -> None
  in

  let rec bi_match_typ rel eq ((l, u) as inst) any t1 t2 =
    if t1 == t2 || SS.mem (t1, t2) !rel
    then Some inst
    else begin
    rel := SS.add (t1, t2) !rel;
    match t1, t2 with
    | Pre, _ | _, Pre ->
      Some inst (* TODO: assert false? *)
    | Any, Any ->
      Some inst
    | _, Any when rel != eq ->
      Some inst
    | Non, Non ->
      Some inst
    | Non, _ when rel != eq ->
      Some inst
    | Named (_n, t1'), t2 ->
      bi_match_typ rel eq inst any t1' t2
    | t1, Named (_n, t2') ->
      bi_match_typ rel eq inst any t1 t2'
    | _, Con (con2, ts2) when flexible con2 ->
      assert (ts2 = []);
      if mentions t1 any || not (denotable t1) then
        None
      else check con2
       (update lub con2 t1 l,
        if rel != eq then u else update glb con2 t1 u)
    | Con (con1, ts1), _ when flexible con1 ->
      assert (ts1 = []);
      if mentions t2 any || not (denotable t2) then
        None
      else check con1
        ((if rel != eq then l else update lub con1 t2 l),
         update glb con1 t2 u)
    | Con (con1, _), Con (con2, _) when flexible con1 && flexible con2 ->
      (* Because we do matching, not unification, we never relate two flexible variables *)
      assert false
    | Con (con1, ts1), Con (con2, ts2) ->
      (match Cons.kind con1, Cons.kind con2 with
      | Def (tbs, t), _ -> (* TBR this may fail to terminate *)
        bi_match_typ rel eq inst any (open_ ts1 t) t2
      | _, Def (tbs, t) -> (* TBR this may fail to terminate *)
        bi_match_typ rel eq inst any t1 (open_ ts2 t)
      | _ when Cons.eq con1 con2 ->
        assert (ts1 = []);
        assert (ts2 = []);
        Some inst
      | Abs (tbs, t), _ when rel != eq ->
        bi_match_typ rel eq inst any (open_ ts1 t) t2
      | _ -> None
      )
    | Con (con1, ts1), t2 ->
      (match Cons.kind con1, t2 with
      | Def (tbs, t), _ -> (* TBR this may fail to terminate *)
        bi_match_typ rel eq inst any (open_ ts1 t) t2
      | Abs (tbs, t), _ when rel != eq ->
        bi_match_typ rel eq inst any (open_ ts1 t) t2
      | _ -> None
      )
    | t1, Con (con2, ts2) ->
      (match Cons.kind con2 with
      | Def (tbs, t) -> (* TBR this may fail to terminate *)
        bi_match_typ rel eq inst any t1 (open_ ts2 t)
      | _ -> None
      )
    | Prim p1, Prim p2 when p1 = p2 ->
      Some inst
    | Prim p1, Prim p2 when rel != eq ->
      if p1 = Nat && p2 = Int then Some inst else None
    | Obj (s1, fs1, tfs1), Obj (s2, fs2, tfs2) ->
      if s1 = s2 then
        match bi_match_fields rel eq inst any fs1 fs2 with
        | None -> None
        | Some inst -> bi_match_typ_fields rel eq inst any tfs1 tfs2
      else None
    | Array t1', Array t2' ->
      bi_match_typ rel eq inst any t1' t2'
    | Opt t1', Opt t2' ->
      bi_match_typ rel eq inst any t1' t2'
    | Weak t1', Weak t2' ->
      bi_match_typ rel eq inst any t1' t2'
    | Prim Null, Opt t2' when rel != eq ->
      Some inst
    | Variant fs1, Variant fs2 ->
      bi_match_tags rel eq inst any fs1 fs2
    | Tup ts1, Tup ts2 ->
      bi_match_list bi_match_typ rel eq inst any ts1 ts2
    | Func (s1, c1, tbs1, t11, t12), Func (s2, c2, tbs2, t21, t22) ->
      if s1 = s2 && c1 = c2 then
      (match bi_match_binds rel eq inst any tbs1 tbs2 with
       | Some (inst, ts) ->
         let any' = List.fold_right
           (fun t -> ConSet.add (fst (as_con t))) ts any
         in
         (match
           bi_match_list bi_match_typ rel eq inst any' (List.map (open_ ts) t21) (List.map (open_ ts) t11)
          with
         | Some inst ->
           bi_match_list bi_match_typ rel eq inst any' (List.map (open_ ts) t12) (List.map (open_ ts) t22)
         | None -> None)
       | None -> None
      )
      else None
    | Async (s1, t11, t12), Async (s2, t21, t22) ->
      if s1 = s2 then
        (match bi_equate_typ rel eq inst any t11 t21  with
         | Some inst ->
           bi_match_typ rel eq inst any t12 t22
         | None -> None)
      else None
    | Mut t1', Mut t2' ->
      bi_equate_typ rel eq inst any t1' t2'
    | _, _ -> None
    end

  and bi_equate_typ rel eq inst any t1 t2 =
    bi_match_typ eq eq inst any t1 t2

  and bi_match_fields rel eq inst any tfs1 tfs2 =
    (* Assume that tfs1 and tfs2 are sorted. *)
    align_fields tfs1 tfs2 |>
    Seq.fold_left (fun inst fs -> match inst with
      | None -> None
      | Some inst -> match fs with
        | Lib.Both(tf1, tf2) -> bi_match_typ rel eq inst any tf1.typ tf2.typ
        | Lib.This(_) -> if rel != eq then Some inst else None
        | Lib.That(_) -> None) (Some inst)

  and bi_match_typ_fields rel eq inst any tfs1 tfs2 =
    (* Assume that tfs1 and tfs2 are sorted. *)
    align_fields tfs1 tfs2 |>
    Seq.fold_left (fun inst fs -> match inst with
      | None -> None
      | Some inst -> match fs with
        | Lib.Both(tf1, tf2) ->
          (* NB: we assume c1, c2 closed *)
          if eq_con tf1.typ tf2.typ then Some inst else None
        | Lib.This(_) -> if rel != eq then Some inst else None
        | Lib.That(_) -> None) (Some inst)

  and bi_match_tags rel eq inst any tfs1 tfs2 =
    (* Assume that tfs1 and tfs2 are sorted. *)
    align_fields tfs1 tfs2 |>
    Seq.fold_left (fun inst fs -> match inst with
      | None -> None
      | Some inst -> match fs with
        | Lib.Both(tf1, tf2) -> bi_match_typ rel eq inst any tf1.typ tf2.typ
        | Lib.This(_) -> None
        | Lib.That(_) -> if rel != eq then Some inst else None) (Some inst);

  and bi_match_binds rel eq inst any tbs1 tbs2 =
    let ts = open_binds tbs2 in
    match bi_match_list (bi_match_bind ts) rel eq inst any tbs2 tbs1 with
    | Some inst -> Some (inst,ts)
    | None -> None

  and bi_match_bind ts rel eq inst any tb1 tb2 =
    bi_match_typ rel eq inst any (open_ ts tb1.bound) (open_ ts tb2.bound)

  in
  bi_match_list_result bi_match_typ

let is_closed ctx t = if is_ctx_empty ctx then true else
  let all_cons = cons_typs [t] in
  ConSet.disjoint ctx.var_set all_cons

(** Raises when [er] is non-empty, optionally with a suggested type instantiation. *)
let maybe_raise_underconstrained ctx env unsolved er =
  let error_msg = ErrorUnderconstrained.to_string er in
  if error_msg = "" then None else
  let error_msg, hint =
    if ConSet.is_empty unsolved then
      (* Future work: fill the unsolved and solved from previous rounds with holes, e.g. <_, Nat, _> *)
      let ts = List.map (fun c -> ConEnv.find c env) ctx.all_vars in
      let inst = String.concat ", " (List.map string_of_typ ts) in
      let hint = Format.asprintf "Hint: Add explicit type instantiation, e.g. <%s>" inst in
      error_msg, Some hint
    else
      error_msg, None
  in
  Some (bimatch error_msg ~hint)

let solve_bounds on_error ctx to_defer l u =
  if debug then Debug.print_solved_bounds l u;
  let unsolved = ref ConSet.empty in
  let er = ErrorUnderconstrained.empty () in
  let env = l |> ConEnv.mapi (fun c lb ->
    let ub = ConEnv.find c u in
    if eq lb ub then
      Some ub
    else if sub lb ub then
      if ConSet.mem c to_defer then begin
        (* Defer solving the type parameter to the next round *)
        unsolved := ConSet.add c !unsolved;
        None
      end else
        Some (choose_under_constrained ctx er lb c ub)
    else
      impossible_over_constrained lb c ub
  ) |> ConEnv.filter_map (fun c o -> o) in
  (* Join the previous solution with the new one *)
  let env = ConEnv.disjoint_union ctx.current_env env in
  Option.iter on_error (maybe_raise_underconstrained ctx env !unsolved er);
  if debug then Debug.print_partial_solution env !unsolved;
  env, !unsolved

(** Solves the given constraints [ts1, ts2] in the given context [ctx].
    Unused type variables can be deferred to the next round.
    [deferred_typs] are types to appear in the constraints of the next round. Used to determine which type variables to defer.
 *)
let solve ctx (ts1, ts2, ats) must_solve =
  if debug then Debug.print_solve ctx (ts1, ts2) must_solve;

  (* Defer solving type variables that can be solved later. More constraints appear in the next round, let them influence as many variables as possible *)
  let to_defer, defer_verify = if must_solve = [] then (ConSet.empty, false) else
    (* Type variables mentioned/used in subtyping constraints *)
    let cons1 = cons_typs ts1 in
    let cons2 = cons_typs ts2 in
    let used = ConSet.inter ctx.var_set (ConSet.union cons1 cons2) in
    let unused = ConSet.diff ctx.var_set used in

    (* Solve only variables that need to be solved now *)
    let to_solve = cons_typs must_solve in
    (* Exclude variables that are not used in the constraints, it is better to raise an error than infer a default bound that could lead to confusing errors *)
    let to_solve = ConSet.diff to_solve unused in
    let to_defer = ConSet.diff ctx.var_set to_solve in
    if debug then Debug.print_variables_to_defer used to_defer (ConSet.inter to_solve ctx.var_set);
    to_defer, not (ConSet.disjoint used to_defer)
  in
  match
    bi_match_typs ctx (ref SS.empty) (ref SS.empty) ctx.bounds ConSet.empty ts1 ts2 ats
  with
  | Ok (l, u) ->
    let env, var_set = solve_bounds raise ctx to_defer l u in
    let remaining = if ConSet.is_empty var_set then empty_ctx env else {
      var_set;
      var_env = ConEnv.restrict var_set ctx.var_env;
      bounds = (
        (* Note that these bounds are not the same as [ctx.bounds], deferred variables might have tigher bounds after solving *)
        ConEnv.restrict var_set l,
        ConEnv.restrict var_set u);
      variances = ConEnv.restrict var_set ctx.variances;
      ret_typ = ctx.ret_typ;
      all_vars = ctx.all_vars;
      current_env = env;
      to_verify = if defer_verify then (List.map (subst env) ts1, List.map (subst env) ts2) else ([], [])
    } in
    let verify_now = if defer_verify then ctx.to_verify else
      let dts1, dts2 = ctx.to_verify in
      (dts1 @ ts1, dts2 @ ts2)
    in
    if verify_inst ~ctx ~remaining verify_now then
      env, remaining
    else begin
      let instantiation = ConEnv.bindings env
        |> List.map (fun (c, t) -> Printf.sprintf "%s := %s" (Cons.name c) (string_of_typ t))
        |> String.concat ", "
      in
      error (Printf.sprintf
        "bug: inferred bad instantiation\n  <%s>\nplease report this error message and, for now, supply an explicit instantiation instead"
        instantiation)
    end
  | Error ((l, u), (t1, t2, at)) ->
    let env, _unsolved = solve_bounds ignore ctx ConSet.empty l u in
    (* Preprocess the substitution for better error messages. Drop variables solved to Any/Non.
      1. Unconstrained variables are solved to Any/Non, don't substitute them, the error was not there.
      2. Matching unrelated types, e.g. Nat and Text, is permitted (resulting in Any/Non bound),
        However, it usually indicates an error. Don't solve these variables.
    *)
    let env = env |> ConEnv.filter (fun c t -> not (eq Any t || eq Non t)) in
    let t1 = subst env t1 in
    let t2 = subst env t2 in
    let reason =
      if is_closed ctx t1 && is_closed ctx t2 then
        let t2 = match t2 with | Named (_, t2) -> t2 | _ -> t2 in
        Some { actual = t1; expected = t2; at }
      else
        None
    in
    let rel = match t2 with
    | Named ("@ret", t2) ->
      Format.asprintf "%a  (for the expected return type) " display_rel (t1, "<:", t2)
    | Named (n, t2) ->
      Format.asprintf "%a  (for argument `%s`) " display_rel (t1, "<:", t2) n
    | t2 ->
      Format.asprintf "%a" display_rel (t1, "<:", t2)
    in
    error ~reason (Format.asprintf "there is no way to satisfy subtyping%s" rel)

let bi_match_subs scope_opt tbs ret_typ =
  (* Create a fresh constructor for each type parameter.
   * These constructors are used as type variables.
   *)
  let ts = open_binds tbs in
  let cs = List.map as_con_var ts in

  (* Extract the constructor for each type variable and create a type variable environment *)
  let var_set = ConSet.of_list cs in
  let var_env = List.fold_left2 (fun acc t tb ->
    let c = as_con_var t in

    (* Check that type parameters have closed bounds *)
    let bound = open_ ts tb.bound in
    if mentions bound var_set then
      fail_open_bound c bound;

    ConEnv.add c { t; bind = tb } acc
  ) ConEnv.empty ts tbs in

  (* Initialize lower and upper bounds for type variables *)
  let l = ConSet.fold (fun c l -> ConEnv.add c Non l) var_set ConEnv.empty in
  let u = ConSet.fold (fun c u -> ConEnv.add c (bound c) u) var_set ConEnv.empty in

  (* Fix the bound of the scope type parameter, if it is there *)
  let l, u = match scope_opt, tbs with
    | Some c, {sort = Scope; _}::tbs ->
      let c0 = as_con_var (List.hd ts) in
      ConEnv.add c0 c l,
      ConEnv.add c0 c u
    | None, {sort = Scope; _}::tbs ->
      error "scope instantiation required but no scope available"
    | _, _ ->
      l,
      u
  in

  (* Compute the variances using the optional return type.
   * Only necessary when the return type is not part of the sub-typing constraints.
   *)
  let ret_typ = Option.map (open_ ts) ret_typ in
  let variances = Variance.variances var_set
    (Option.value ~default:Any ret_typ)
  in
  let ctx = { var_set; var_env; bounds = (l, u); variances; ret_typ; all_vars = cs; current_env = ConEnv.empty; to_verify = ([], [])} in
  fun subs ~must_solve ->
    let must_solve = List.map (open_ ts) must_solve in
    let ts1 = List.map (fun (t1, _, _) -> open_ ts t1) subs in
    let ts2 = List.map (fun (_, t2, _) -> open_ ts t2) subs in
    let ats = List.map (fun (_, _, at) -> at) subs in
    let env, remaining = solve ctx (ts1, ts2, ats) must_solve in
    List.map (subst env) ts, remaining

let finalize ts1 ctx subs =
  if is_ctx_empty ctx then begin
    assert (subs = []);
    ts1, ConEnv.empty
  end else begin
    (* Solve the 2nd round of sub-type problems *)
    let env, remaining = solve ctx (Lib.List.split3 subs) [] in

    (* The 2nd round should not leave any remaining type variables *)
    assert (is_ctx_empty remaining);

    (* create the final solution *)
    List.map (fun c -> ConEnv.find c env) ctx.all_vars, env
  end

let fail_when_types_are_not_closed remaining typs = if is_ctx_empty remaining then () else
  let all_cons = cons_typs typs in
  let open_con_set = ConSet.inter remaining.var_set all_cons in
  if not (ConSet.is_empty open_con_set) then
    let message = Printf.sprintf "`%s` cannot be inferred." (String.concat "`, `" (List.map Cons.name (ConSet.elements open_con_set))) in
    error message
