open Mo_types
open Ir_def

module E = Env
open Source
module Ir = Ir
open Ir
module T = Type
open T

(* Erase type fields from object types *)

module ConRenaming = E.Make(struct type t = T.con let compare = Cons.compare end)

let transform prog =

  (* the state *)
  let con_renaming = ref ConRenaming.empty

  (* maps constructors to new constructors (name name, new stamp, new kind)
     it is initialized with the type constructors defined outside here, which are
     not rewritten.

     If we run this translation on two program fragments (e.g. prelude and program)
     we would have to pass down the `con_renaming`. But this is simply the right thing
     to do for a pass that changes the context.

     Eventually, pipeline will allow us to pass the con_renaming to downstream program
     fragments, then we would simply start with an empty con_renaming and the prelude.

     (c.f. async.ml.)
  *)
  in

  (* Mostly boiler-plate homomorphic translation *)
  let rec t_typ t =
    match t with
    (* The only interesting cases *)
    | Obj (s, fs, _) ->
      Obj (s, List.map (fun f -> t_field f) fs, [])
    | Named (n , t) -> (* erased! *)
      t_typ t
    | T.Prim _
    | Var _ -> t
    | Con (c, ts) ->
      Con (t_con c, List.map t_typ ts)
    | Array t -> Array (t_typ t)
    | Tup ts -> Tup (List.map t_typ ts)
    | Func (s, c, tbs, ts1, ts2) ->
      Func (s, c, List.map t_bind tbs, List.map t_typ ts1, List.map t_typ ts2)
    | Opt t -> Opt (t_typ t)
    | Variant fs -> Variant (List.map t_field fs)
    | Async (s, t1, t2) -> Async (s, t_typ t1, t_typ t2)
    | Mut t -> Mut (t_typ t)
    | Any -> Any
    | Non -> Non
    | Pre -> Pre
    | Weak t -> Weak (t_typ t)

  and t_bind tb =
    { tb with bound = t_typ tb.bound }

  and t_binds typbinds = List.map t_bind typbinds

  and t_kind k =
    match k with
    | T.Abs (typ_binds,typ) ->
      T.Abs (t_binds typ_binds, t_typ typ)
    | T.Def (typ_binds,typ) ->
      T.Def (t_binds typ_binds, t_typ typ)

  and t_con c =
    match Cons.kind c with
    | T.Def ([], T.Prim _) -> c
    | _ ->
      match ConRenaming.find_opt c (!con_renaming) with
      | Some c' -> c'
      | None ->
        let clone = Cons.clone c (Abs ([], Pre)) in
        con_renaming := ConRenaming.add c clone (!con_renaming);
        (* Need to extend con_renaming before traversing the kind *)
        Type.set_kind clone (t_kind (Cons.kind c));
        clone

  and t_prim p = Ir.map_prim t_typ Fun.id p

  and t_field {lab; typ; src} =
    { lab; typ = t_typ typ; src }
  in

  let rec t_exp (exp : exp) =
    { it = t_exp' exp;
      note = Note.{ def with
        typ = t_typ exp.note.typ;
        eff = exp.note.eff
      };
      at = exp.at;
    }
  and t_exp' (exp : exp) =
    let exp' = exp.it in
    match exp' with
    | LitE _
    | VarE _ -> exp'
    | AssignE (exp1, exp2) ->
      AssignE (t_lexp exp1, t_exp exp2)
    | PrimE (p, exps) ->
      PrimE (t_prim p, List.map t_exp exps)
    | BlockE b ->
      BlockE (t_block b)
    | IfE (exp1, exp2, exp3) ->
      IfE (t_exp exp1, t_exp exp2, t_exp exp3)
    | SwitchE (exp1, cases) ->
      SwitchE (t_exp exp1, List.map t_case cases)
    | LoopE exp1 ->
      LoopE (t_exp exp1)
    | LabelE (id, typ, exp1) ->
      LabelE (id, t_typ typ, t_exp exp1)
    | AsyncE (s, tb, exp1, typ) ->
      AsyncE (s, t_typ_bind tb, t_exp exp1, t_typ typ)
    | TryE (exp1, cases, vt) ->
      TryE (t_exp exp1, List.map t_case cases, vt)
    | DeclareE (id, typ, exp1) ->
      DeclareE (id, t_typ typ, t_exp exp1)
    | DefineE (id, mut ,exp1) ->
      DefineE (id, mut, t_exp exp1)
    | FuncE (x, s, c, typbinds, args, ret_tys, exp) ->
      FuncE (x, s, c, t_typ_binds typbinds, t_args args, List.map t_typ ret_tys, t_exp exp)
    | ActorE (ds, fs, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, typ) ->
      ActorE (t_decs ds, t_fields fs,
       {meta;
        preupgrade = t_exp preupgrade;
        postupgrade = t_exp postupgrade;
        heartbeat = t_exp heartbeat;
        timer = t_exp timer;
        inspect = t_exp inspect;
        low_memory = t_exp low_memory;
        stable_record = t_exp stable_record;
        stable_type = {
            pre = t_typ stable_type.pre;
            post = t_typ stable_type.post
        }
       },
       t_typ typ)

    | NewObjE (sort, ids, t) ->
      NewObjE (sort, t_fields ids, t_typ t)
    | SelfCallE _ -> assert false

  and t_case {it = {pat; exp}; at; note} =
    { it = {pat = t_pat pat; exp = t_exp exp}; at; note}

  and t_lexp lexp =
    { it = t_lexp' lexp.it;
      note = t_typ lexp.note;
      at = lexp.at;
    }
  and t_lexp' (lexp' : lexp') =
    match lexp' with
    | VarLE _ -> lexp'
    | DotLE (exp1, id) ->
      DotLE (t_exp exp1, id)
    | IdxLE (exp1, exp2) ->
      IdxLE (t_exp exp1, t_exp exp2)

  and t_dec dec = { dec with it = t_dec' dec.it }

  and t_dec' dec' =
    match dec' with
    | LetD (pat,exp) -> LetD (t_pat pat,t_exp exp)
    | VarD (id, t, exp) -> VarD (id, t_typ t, t_exp exp)
    | RefD (id, t, lexp) -> RefD (id, t_typ t, t_lexp lexp)

  and t_decs decs = List.map t_dec decs

  and t_block (ds, exp) = (t_decs ds, t_exp exp)

  and t_fields fs =
    List.map (fun f -> { f with note = t_typ f.note }) fs

  and t_args as_ = List.map t_arg as_

  and t_arg a = { a with note = t_typ a.note }

  and t_pat pat =
    { pat with
      it = t_pat' pat.it;
      note = t_typ pat.note }

  and t_pat' pat =
    match pat with
    | WildP
    | LitP _
    | VarP _ ->
      pat
    | TupP pats ->
      TupP (List.map t_pat pats)
    | ObjP pfs ->
      ObjP (map_obj_pat t_pat pfs)
    | OptP pat1 ->
      OptP (t_pat pat1)
    | TagP (i, pat1) ->
      TagP (i, t_pat pat1)
    | AltP (pat1, pat2) ->
      AltP (t_pat pat1, t_pat pat2)

  and t_typ_bind' tb =
    { tb with con = t_con tb.con; bound = t_typ tb.bound }

  and t_typ_bind typ_bind =
    { typ_bind with it = t_typ_bind' typ_bind.it }

  and t_typ_binds typbinds = List.map t_typ_bind typbinds

  and t_comp_unit = function
    | LibU _ -> raise (Invalid_argument "cannot compile library")
    | ProgU ds -> ProgU (t_decs ds)
    | ActorU (args_opt, ds, fs, {meta; preupgrade; postupgrade; heartbeat; timer; inspect; low_memory; stable_record; stable_type}, t) ->
      ActorU (Option.map t_args args_opt, t_decs ds, t_fields fs,
        { meta;
          preupgrade = t_exp preupgrade;
          postupgrade = t_exp postupgrade;
          heartbeat = t_exp heartbeat;
          timer = t_exp timer;
          inspect = t_exp inspect;
          low_memory = t_exp low_memory;
          stable_record = t_exp stable_record;
          stable_type = {
            pre = t_typ stable_type.pre;
            post = t_typ stable_type.post
          }
        },
        t_typ t)
  and t_prog (cu, flavor) = (t_comp_unit cu, { flavor with has_typ_field = false } )
in
  t_prog prog
