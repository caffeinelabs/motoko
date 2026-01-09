open Mo_def
open Syntax
open Source

let rec over_exp (f : exp -> exp) (exp : exp) : exp = match exp.it with
  | HoleE (s, exp1) -> f { exp with it = HoleE (s, ref (over_exp f (!exp1))) }
  | ImportE _ | ImplicitLibE _ | PrimE _ | VarE _ | LitE _ | ActorUrlE _ -> f exp
  | UnE (x, y, exp1) -> f { exp with it = UnE (x, y, over_exp f exp1) }
  | ShowE (x, exp1) -> f { exp with it = ShowE (x, over_exp f exp1) }
  | ToCandidE exps ->  f { exp with it = ToCandidE (List.map (over_exp f) exps) }
  | FromCandidE (exp1) -> f { exp with it = FromCandidE (over_exp f exp1) }
  | ProjE (exp1, x) -> f { exp with it = ProjE (over_exp f exp1, x) }
  | OptE exp1 -> f { exp with it = OptE (over_exp f exp1) }
  | DoOptE exp1 -> f { exp with it = DoOptE (over_exp f exp1) }
  | BangE exp1 -> f { exp with it = BangE (over_exp f exp1) }
  | TagE (x, exp1) -> f { exp with it = TagE (x, over_exp f exp1) }
  | DotE (exp1, x, n) -> f { exp with it = DotE (over_exp f exp1, x, ref (Option.map (over_exp f) !n)) }
  | NotE exp1 -> f { exp with it = NotE (over_exp f exp1) }
  | AssertE (how, exp1) -> f { exp with it = AssertE (how, over_exp f exp1) }
  | LabelE (x, y, exp1) -> f { exp with it = LabelE (x, y, over_exp f exp1) }
  | BreakE (k, i, exp1) -> f { exp with it = BreakE (k, i, over_exp f exp1) }
  | RetE exp1 -> f { exp with it = RetE (over_exp f exp1) }
  | AnnotE (exp1, x) -> f { exp with it = AnnotE (over_exp f exp1, x) }
  | AsyncE (par, s, tb, exp1) -> f { exp with it = AsyncE (Option.map (over_exp f) par, s, tb, over_exp f exp1) }
  | AwaitE (s, exp1) -> f { exp with it = AwaitE (s, over_exp f exp1) }
  | ThrowE exp1 -> f { exp with it = ThrowE (over_exp f exp1) }
  | BinE (x, exp1, y, exp2) ->
     f { exp with it = BinE (x, over_exp f exp1, y, over_exp f exp2) }
  | IdxE (exp1, exp2) ->
     f { exp with it = IdxE (over_exp f exp1, over_exp f exp2) }
  | RelE (x, exp1, y, exp2) ->
     f { exp with it = RelE (x, over_exp f exp1, y, over_exp f exp2) }
  | AssignE (exp1, exp2) ->
     f { exp with it = AssignE (over_exp f exp1, over_exp f exp2) }
  | CallE (par_opt, exp1, x, (s, exp2)) ->
     f { exp with it = CallE (Option.map (over_exp f) par_opt, over_exp f exp1, x, (s, ref (over_exp f !exp2))) }
  | AndE (exp1, exp2) ->
     f { exp with it = AndE (over_exp f exp1, over_exp f exp2) }
  | OrE (exp1, exp2) ->
     f { exp with it = OrE (over_exp f exp1, over_exp f exp2) }
  | WhileE (exp1, exp2, flags) ->
     f { exp with it = WhileE (over_exp f exp1, over_exp f exp2, flags) }
  | LoopE (exp1, exp2_opt, flags) ->
     f { exp with it = LoopE (over_exp f exp1, Option.map (over_exp f) exp2_opt, flags) }
  | ForE (x, exp1, exp2, flags) ->
     f { exp with it = ForE (x, over_exp f exp1, over_exp f exp2, flags) }
  | DebugE exp1 ->
     f { exp with it = DebugE (over_exp f exp1) }
  | TupE exps ->
     f { exp with it = TupE (List.map (over_exp f) exps) }
  | ArrayE (x, exps) ->
     f { exp with it = ArrayE (x, List.map (over_exp f) exps) }
  | BlockE ds ->
     f { exp with it = BlockE (List.map (over_dec f) ds) }
  | ObjBlockE (eo, s, t, dfs) ->
     f { exp with it = ObjBlockE (Option.map (over_exp f) eo, s, t, List.map (over_dec_field f) dfs) }
  | ObjE (bases, efs) ->
     f { exp with it = ObjE (List.map (over_exp f) bases, List.map (over_exp_field f) efs) }
  | IfE (exp1, exp2, exp3) ->
     f { exp with it = IfE(over_exp f exp1, over_exp f exp2, over_exp f exp3) }
  | TryE (exp1, cases, exp2_opt) ->
     f { exp with it = TryE (over_exp f exp1, List.map (over_case f) cases, Option.map (over_exp f) exp2_opt) }
  | SwitchE (exp1, cases) ->
     f { exp with it = SwitchE (over_exp f exp1, List.map (over_case f) cases) }
  | FuncE (name, sort_pat, typ_binds, pat, typ_opt, sugar, exp1) ->
     f { exp with it = FuncE (name, sort_pat, typ_binds, pat, typ_opt, sugar, over_exp f exp1) }
  | IgnoreE exp1 ->
     f { exp with it = IgnoreE (over_exp f exp1)}

and over_dec (f : exp -> exp) (d : dec) : dec = match d.it with
  | TypD _ -> d
  | ExpD e -> { d with it = ExpD (over_exp f e)}
  | VarD (x, e) ->
     { d with it = VarD (x, over_exp f e)}
  | LetD (x, e, fail) ->
     { d with it = LetD (x, over_exp f e, Option.map (over_exp f) fail)}
  | ClassD (e_o, sp, s, cid, tbs, p, t_o, id, dfs) ->
     { d with it = ClassD (Option.map (over_exp f) e_o, sp, s, cid, tbs, p, t_o, id, List.map (over_dec_field f) dfs)}
  | MixinD (p, dfs) ->
     { d with it = MixinD (p, List.map (over_dec_field f) dfs)}
  | IncludeD (id, e, n) -> { d with it = IncludeD(id, over_exp f e, n) }

and over_dec_field (f : exp -> exp) (df : dec_field) : dec_field =
  { df with it = { df.it with dec = over_dec f df.it.dec } }

and over_exp_field (f : exp -> exp) (ef : exp_field) : exp_field =
  { ef with it = { ef.it with exp = over_exp f ef.it.exp } }

and over_case (f : exp -> exp) (case : case) : case =
  { case with it = { case.it with exp = over_exp f case.it.exp } }

and over_prog (f : exp -> exp) (prog : prog) : prog =
  { prog with it = List.map (over_dec f) prog.it }
