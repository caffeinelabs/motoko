open Mo_types
open Mo_values

open Source
open Syntax
open Wasm.Sexpr

type type_rep_config =
  | Without_type_rep
  | With_type_rep of Field_sources.srcs_map option

module type Config = sig
  val include_sources : bool
  val include_types : bool
  val include_type_rep : type_rep_config
  val include_docs : Trivia.trivia_info Trivia.PosHashtbl.t option
  val include_parenthetical : bool
  val main_file : string option
end

module Default : Config = struct
  let include_sources = false
  let include_types = false
  let include_type_rep = Without_type_rep
  let include_parenthetical = true (* false for vscode *)
  let include_docs = None
  let main_file = None
end

module Type_pretty = Mo_types.Type.MakePretty (Mo_types.Type.ElideStamps)

module Make (Cfg : Config) = struct
  let ($$) head inner = Node (head, inner)

  let pos p =
    let file = match Cfg.main_file with
    | Some f when f <> p.file -> p.file
    | _ -> ""
    in "Pos" $$ [Atom file; Atom (string_of_int p.line); Atom (string_of_int p.column)]
  let source at it = if Cfg.include_sources && at <> Source.no_region then "@" $$ [pos at.left; pos at.right; it] else it

  let typ t = Atom (Type_pretty.string_of_typ t)

  let trivia at it =
    match Cfg.include_docs with
    | Some table ->
      let rec lookup_trivia (line, column) =
        Trivia.PosHashtbl.find_opt table Trivia.{ line; column }
      and find_trivia (parser_pos : Source.region) : Trivia.trivia_info =
        lookup_trivia Source.(parser_pos.left.line, parser_pos.left.column) |> Option.get
      in
      (match Trivia.doc_comment_of_trivia_info (find_trivia at) with
      | Some s -> "*" $$ [Atom s; it]
      | None -> it)
    | None -> it

  let eff (eff : Mo_types.Type.eff) = match eff with
  | Mo_types.Type.Triv -> Atom "Triv"
  | Mo_types.Type.Await -> Atom "Await"

  let annot_typ t it =
    if Cfg.include_types
    then ":" $$ [it; typ t]
    else it

  let id i = source i.at ("ID" $$ [Atom i.it])
  let tag i = Atom ("#" ^ i.it)

  let obj_sort s = match s.it with
    | Type.Object -> Atom "Object"
    | Type.Actor -> Atom "Actor"
    | Type.Mixin -> Atom "Mixin"
    | Type.Module -> Atom "Module"
    | Type.Memory -> Atom "Memory"

  let rec exp e = source e.at (annot_typ e.note.note_typ (match e.it with
    | HoleE (_, e) -> "HoleE" $$ [exp !e]
    | VarE x              -> "VarE"      $$ [id x]
    | LitE l              -> "LitE"      $$ [lit !l]
    | ActorUrlE e         -> "ActorUrlE" $$ [exp e]
    | UnE (ot, uo, e)     -> "UnE"       $$ [operator_type !ot; Arrange_ops.unop uo; exp e]
    | BinE (ot, e1, bo, e2) -> "BinE"    $$ [operator_type !ot; exp e1; Arrange_ops.binop bo; exp e2]
    | RelE (ot, e1, ro, e2) -> "RelE"    $$ [operator_type !ot; exp e1; Arrange_ops.relop ro; exp e2]
    | ShowE (ot, e)       -> "ShowE"     $$ [operator_type !ot; exp e]
    | ToCandidE es        -> "ToCandidE"   $$ exps es
    | FromCandidE e       -> "FromCandidE" $$ [exp e]
    | TupE es             -> "TupE"      $$ exps es
    | ProjE (e, i)        -> "ProjE"     $$ [exp e; Atom (string_of_int i)]
    | ObjBlockE (eo, s, nt, dfs) -> "ObjBlockE" $$
                                               parenthetical eo
                                               [obj_sort s;
                                                match nt with
                                                | None, None -> Atom "_"
                                                | None, Some t -> typ t
                                                | Some id, Some t -> id.it $$ [Atom ":"; typ t]
                                                | Some id, None -> Atom id.it
                                                ] @ List.map dec_field dfs
    | ObjE ([], efs)      -> "ObjE"      $$ List.map exp_field efs
    | ObjE (bases, efs)   -> "ObjE"      $$ exps bases @ [Atom "with"] @ List.map exp_field efs
    | DotE (e, x, ol)     -> "DotE"      $$ [exp e; match !ol with None -> id x | Some e -> exp e]
    | AssignE (e1, e2)    -> "AssignE"   $$ [exp e1; exp e2]
    | ArrayE (m, es)      -> "ArrayE"    $$ [mut m] @ exps es
    | IdxE (e1, e2)       -> "IdxE"      $$ [exp e1; exp e2]
    | FuncE (x, sp, tp, p, t, sugar, e') ->
      "FuncE" $$ [
        Atom (Type.string_of_typ e.note.note_typ);
        shared_pat sp;
        Atom x] @
        List.map typ_bind tp @ [
        pat p;
        (match t with None -> Atom "_" | Some t -> typ t);
        Atom (if sugar then "" else "=");
        exp e'
      ]
    | CallE (par_opt, e1, ts, (_, e2)) -> "CallE" $$ parenthetical par_opt ([exp e1] @ inst ts @ [exp !e2])
    | BlockE ds           -> "BlockE"  $$ List.map dec ds
    | NotE e              -> "NotE"    $$ [exp e]
    | AndE (e1, e2)       -> "AndE"    $$ [exp e1; exp e2]
    | OrE (e1, e2)        -> "OrE"     $$ [exp e1; exp e2]
    | IfE (e1, e2, e3)    -> "IfE"     $$ [exp e1; exp e2; exp e3]
    | SwitchE (e, cs)     -> "SwitchE" $$ [exp e] @ List.map case cs
    | WhileE (e1, e2, _)     -> "WhileE"  $$ [exp e1; exp e2]
    | LoopE (e1, None, _)    -> "LoopE"   $$ [exp e1]
    | LoopE (e1, Some e2, _) -> "LoopE"   $$ [exp e1; exp e2]
    | ForE (p, e1, e2, _)    -> "ForE"    $$ [pat p; exp e1; exp e2]
    | LabelE (i, t, e)       -> "LabelE"  $$ [id i; typ t; exp e]
    | DebugE e            -> "DebugE"  $$ [exp e]
    | BreakE (_, Some i, e) -> "BreakE"  $$ [id i; exp e]
    | BreakE (_, None, e)   -> "BreakE"  $$ [exp e]
    | RetE e              -> "RetE"    $$ [exp e]
    | AsyncE (par_opt, Type.Fut, tb, e) -> "AsyncE" $$ parenthetical par_opt [typ_bind tb; exp e]
    | AsyncE (None, Type.Cmp, tb, e) -> "AsyncE*" $$ [typ_bind tb; exp e]
    | AsyncE (Some _ , Type.Cmp, tb, e) -> assert false;
    | AwaitE (Type.AwaitFut false, e)   -> "AwaitE" $$ [exp e]
    | AwaitE (Type.AwaitFut true, e)    -> "AwaitE?" $$ [exp e]
    | AwaitE (Type.AwaitCmp, e)  -> "AwaitE*" $$ [exp e]
    | AssertE (Runtime, e)       -> "AssertE" $$ [exp e]
    | AnnotE (e, t)       -> "AnnotE"  $$ [exp e; typ t]
    | OptE e              -> "OptE"    $$ [exp e]
    | DoOptE e            -> "DoOptE"  $$ [exp e]
    | BangE e             -> "BangE"   $$ [exp e]
    | TagE (i, e)         -> "TagE"    $$ [id i; exp e]
    | PrimE p             -> "PrimE"   $$ [Atom p]
    | ImportE (f, _fp)    -> "ImportE" $$ [Atom f]
    | ImplicitLibE l      -> "ImplicitLibE" $$ [Atom l]
    | ThrowE e            -> "ThrowE"  $$ [exp e]
    | TryE (e, cs, None)  -> "TryE"    $$ [exp e] @ List.map catch cs
    | TryE (e, cs, Some f)-> "TryE"    $$ [exp e] @ List.map catch cs @ Atom ";" :: [exp f]
    | IgnoreE e           -> "IgnoreE" $$ [exp e]))

  and exps es = List.map exp es

  and inst inst = match inst.it with
    | None -> []
    | Some (false, ts) -> List.map typ ts
    | Some (true, ts) -> Atom "system" :: List.map typ ts

  and pat p = source p.at (annot_typ p.note (match p.it with
    | WildP           -> Atom "WildP"
    | VarP x          -> "VarP"       $$ [id x]
    | TupP ps         -> "TupP"       $$ List.map pat ps
    | ObjP ps         -> "ObjP"       $$ List.map pat_field ps
    | AnnotP (p, t)   -> "AnnotP"     $$ [pat p; typ t]
    | LitP l          -> "LitP"       $$ [lit !l]
    | SignP (uo, l)   -> "SignP"      $$ [Arrange_ops.unop uo ; lit !l]
    | OptP p          -> "OptP"       $$ [pat p]
    | TagP (i, p)     -> "TagP"       $$ [tag i; pat p]
    | AltP (p1, p2)   -> "AltP"       $$ [pat p1; pat p2]
    | ParP p          -> "ParP"       $$ [pat p]))

  and lit = function
    | NullLit       -> Atom "NullLit"
    | BoolLit true  -> "BoolLit"   $$ [ Atom "true" ]
    | BoolLit false -> "BoolLit"   $$ [ Atom "false" ]
    | NatLit n      -> "NatLit"    $$ [ Atom (Numerics.Nat.to_pretty_string n) ]
    | Nat8Lit n     -> "Nat8Lit"   $$ [ Atom (Numerics.Nat8.to_pretty_string n) ]
    | Nat16Lit n    -> "Nat16Lit"  $$ [ Atom (Numerics.Nat16.to_pretty_string n) ]
    | Nat32Lit n    -> "Nat32Lit"  $$ [ Atom (Numerics.Nat32.to_pretty_string n) ]
    | Nat64Lit n    -> "Nat64Lit"  $$ [ Atom (Numerics.Nat64.to_pretty_string n) ]
    | IntLit i      -> "IntLit"    $$ [ Atom (Numerics.Int.to_pretty_string i) ]
    | Int8Lit i     -> "Int8Lit"   $$ [ Atom (Numerics.Int_8.to_pretty_string i) ]
    | Int16Lit i    -> "Int16Lit"  $$ [ Atom (Numerics.Int_16.to_pretty_string i) ]
    | Int32Lit i    -> "Int32Lit"  $$ [ Atom (Numerics.Int_32.to_pretty_string i) ]
    | Int64Lit i    -> "Int64Lit"  $$ [ Atom (Numerics.Int_64.to_pretty_string i) ]
    | FloatLit f    -> "FloatLit"  $$ [ Atom (Numerics.Float.to_pretty_string f) ]
    | CharLit c     -> "CharLit"   $$ [ Atom (string_of_int c) ]
    | TextLit t     -> "TextLit"   $$ [ Atom t ]
    | BlobLit b     -> "BlobLit"   $$ [ Atom b ]
    | PreLit (s, p)  -> "PreLit"    $$ [ Atom s; Arrange_type.prim p ]

  and case c = source c.at ("case" $$ [pat c.it.pat; exp c.it.exp])

  and catch c = "catch" $$ [pat c.it.pat; exp c.it.exp]

  and pat_field pf = source pf.at (match pf.it with
    | ValPF(id, p) -> "ValPF" $$ [Atom id.it; pat p]
    | TypPF(id) -> "TypPF" $$ [Atom id.it])

  (* conditionally include parenthetical to avoid breaking lsp *)
  and parenthetical eo sexps =
    if Cfg.include_parenthetical then
      (match eo with None -> Atom "_" | Some e -> exp e) ::
      sexps
    else sexps


  and shared_pat sp = match sp.it with
    | Type.Local -> Atom "Local"
    | Type.Shared (Type.Write, p) -> "Shared" $$ [pat p]
    | Type.Shared (Type.Query, p) -> "Query" $$ [pat p]
    | Type.Shared (Type.Composite, p) -> "Composite" $$ [pat p]

  and func_sort s = match s.it with
    | Type.Local -> Atom "Local"
    | Type.Shared Type.Write -> Atom "Shared"
    | Type.Shared Type.Query -> Atom "Query"
    | Type.Shared Type.Composite -> Atom "Composite"

  and mut m = match m.it with
    | Const -> Atom "Const"
    | Var   -> Atom "Var"

  and vis v = match v.it with
    | Public None -> Atom "Public"
    | Public (Some m) -> "Public" $$ [Atom m]
    | Private -> Atom "Private"
    | System -> Atom "System"

  and stab s_opt = match s_opt with
    | None -> Atom "(Flexible)"
    | Some s ->
      (match s.it with
      | Flexible -> Atom "Flexible"
      | Stable -> Atom "Stable")

  and typ_field (tf : typ_field) = source tf.at (match tf.it with
    | ValF (lab, t, m) -> "ValF" $$ [id lab; typ t; mut m]
    | TypF (lab, tbs, t) -> "TypF" $$ id lab :: List.map typ_bind tbs @ [typ t])

  and typ_item ((id, ty) : typ_item) =
    match id with
    | None -> [typ ty]
    | Some { it;_ } -> [Atom it; typ ty]

  and typ_tag (tt : typ_tag)
    = source tt.at (tt.it.tag.it $$ [typ tt.it.typ])

  and typ_bind (tb : typ_bind)
    = source tb.at (tb.it.var.it $$ [typ tb.it.bound])

  and dec_field (df : dec_field)
    = trivia df.at (source df.at ("DecField" $$ [dec df.it.dec; vis df.it.vis; stab df.it.stab]))

  and exp_field (ef : exp_field)
    = source ef.at ("ExpField" $$ [mut ef.it.mut; id ef.it.id; exp ef.it.exp])

  and operator_type t = Atom (Type.string_of_typ t)

  and path p = match p.it with
    | IdH i -> "IdH" $$ [id i]
    | DotH (p, i) -> "DotH" $$ [path p; id i]

  and typ t = source t.at (annot_typ t.note (match t.it with
  | PathT (p, ts) -> "PathT" $$ [path p] @ List.map typ ts
  | PrimT p -> "PrimT" $$ [Atom p]
  | ObjT (s, ts) -> "ObjT" $$ [obj_sort s] @ List.map typ_field ts
  | ArrayT (m, t) -> "ArrayT" $$ [mut m; typ t]
  | OptT t -> "OptT" $$ [typ t]
  | VariantT cts -> "VariantT" $$ List.map typ_tag cts
  | TupT ts -> "TupT" $$ List.concat_map typ_item ts
  | FuncT (s, tbs, at, rt) -> "FuncT" $$ [func_sort s] @ List.map typ_bind tbs @ [ typ at; typ rt]
  | AsyncT (Type.Fut, t1, t2) -> "AsyncT" $$ [typ t1; typ t2]
  | AsyncT (Type.Cmp, t1, t2) -> "AsyncT*" $$ [typ t1; typ t2]
  | AndT (t1, t2) -> "AndT" $$ [typ t1; typ t2]
  | OrT (t1, t2) -> "OrT" $$ [typ t1; typ t2]
  | ParT t -> "ParT" $$ [typ t]
  | NamedT (id, t) -> "NamedT" $$ [Atom id.it; typ t]
  | WeakT t -> "WeakT" $$ [typ t]))

  and dec d = trivia d.at (source d.at (match d.it with
    | ExpD e -> "ExpD" $$ [exp e]
    | LetD (p, e, Some f) -> "LetD" $$ [pat p; exp e; exp f]
    | LetD (p, e, None) -> "LetD" $$ [pat p; exp e]
    | VarD (x, e) -> "VarD" $$ [id x; exp e]
    | TypD (x, tp, t) ->
      "TypD" $$ [id x] @ List.map typ_bind tp @ [typ t]
    | ClassD (eo, sp, s, x, tp, p, rt, i, dfs) ->
      "ClassD" $$
        parenthetical eo
        (shared_pat sp :: id x :: List.map typ_bind tp @ [
        pat p;
        (match rt with None -> Atom "_" | Some t -> typ t);
        obj_sort s;
        id i
      ] @ List.map dec_field dfs)
    | MixinD (_, dfs) -> "MixinD" $$ List.map dec_field dfs
    | IncludeD (i, e, _) -> "IncludeD" $$ [id i; exp e]))

  and prog p = "Prog" $$ List.map dec p.it
end

module type S = module type of Make (Default)

(* Defaults *)
include Make (Default)
