module Arrange = Mo_def.Arrange
module Js = Js_of_ocaml.Js
module Numerics = Mo_values.Numerics
module Operator = Mo_values.Operator
module Syntax = Mo_def.Syntax
module Type = Mo_types.Type
module Type_pretty = Mo_types.Type.MakePretty (Mo_types.Type.ElideStamps)

module type Config = Mo_def.Arrange.Config
module type TypConfig = Mo_types.Arrange_type.Config

(* AST to JSON conversion *)
module Make (Cfg : Config) = struct
  (* Creates JSON object for AST node *)
  let to_js_object (name : string) (args : Js.Unsafe.any array) =
    Js.Unsafe.coerce
      (object%js
         val name = Js.string name
         val args = Js.array args
      end)

  (* Converts string to JSON-string as [Js.Unsafe.any] *)
  let js_string (s : string) : Js.Unsafe.any =
    s |> Js.string |> Js.Unsafe.coerce

  let srcs_tbl =
    let open Arrange in
    match Cfg.include_type_rep with
    | With_type_rep s -> s
    | Without_type_rep -> None

  let syntax_pos_js p =
    let open Source in
    let file =
      match Cfg.main_file with Some f when f <> p.file -> p.file | _ -> ""
    in
    to_js_object "Pos"
      (Array.map js_string
         [| file; string_of_int p.line; string_of_int p.column |])

  let type_pos_js p =
    to_js_object "Pos"
      [|
        js_string (string_of_int p.Source.line);
        js_string (string_of_int p.Source.column);
      |]

  let prim_js p =
    let open Type in
    (match p with
    | Null -> "Null"
    | Bool -> "Bool"
    | Nat -> "Nat"
    | Nat8 -> "Nat8"
    | Nat16 -> "Nat16"
    | Nat32 -> "Nat32"
    | Nat64 -> "Nat64"
    | Int -> "Int"
    | Int8 -> "Int8"
    | Int16 -> "Int16"
    | Int32 -> "Int32"
    | Int64 -> "Int64"
    | Float -> "Float"
    | Char -> "Char"
    | Text -> "Text"
    | Blob -> "Blob"
    | Error -> "Error"
    | Principal -> "Principal"
    | Region -> "Region")
    |> js_string

  let obj_sort_js o =
    let open Type in
    (match o with
    | Object -> "Object"
    | Actor -> "Actor"
    | Module -> "Module"
    | Memory -> "Memory"
    | Mixin -> "Mixin")
    |> js_string

  let func_sort_js s =
    let open Type in
    (match s with
    | Local -> "Local"
    | Shared Write -> "Shared"
    | Shared Query -> "Shared Query"
    | Shared Composite -> "Shared Composite")
    |> js_string

  let control_js c =
    let open Type in
    (match c with
    | Returns -> "Returns"
    | Promises -> "Promises"
    | Replies -> "Replies")
    |> js_string

  let region_js r =
    let open Source in
    let filename = r.left.file in
    to_js_object "@@"
      [| js_string filename; type_pos_js r.left; type_pos_js r.right |]

  let mut_js m =
    let open Source in
    let open Syntax in
    js_string (match m.it with Const -> "Const" | Var -> "Var")

  let rec typ_js =
    let open Type in
    function
    | Var (s, i) ->
        to_js_object "Var" (Array.map js_string [| s; string_of_int i |])
    | Con (c, ts) ->
        to_js_object "Con"
          (js_string (Type.string_of_con c) :: List.map typ_js ts
          |> Array.of_list)
    | Prim p -> to_js_object "Prim" [| prim_js p |]
    | Obj (s, tfs) ->
        to_js_object "Obj"
          ([ obj_sort_js s ] @ List.map field_js tfs |> Array.of_list)
    | Array t -> to_js_object "Array" [| typ_js t |]
    | Opt t -> to_js_object "Opt" [| typ_js t |]
    | Variant tfs ->
        to_js_object "Variant" (List.map field_js tfs |> Array.of_list)
    | Tup ts -> to_js_object "Tup" (List.map typ_js ts |> Array.of_list)
    | Func (s, c, tbs, at, rt) ->
        to_js_object "Func"
          ([ func_sort_js s; control_js c ]
           @ List.map bind_js tbs
           @ [
               to_js_object "" (List.map typ_js at |> Array.of_list);
               to_js_object "" (List.map typ_js rt |> Array.of_list);
             ]
          |> Array.of_list)
    | Async (Fut, t1, t2) ->
        to_js_object "Async" (Array.map typ_js [| t1; t2 |])
    | Async (Cmp, t1, t2) ->
        to_js_object "Async*" (Array.map typ_js [| t1; t2 |])
    | Mut t -> to_js_object "Mut" [| typ_js t |]
    | Any -> js_string "Any"
    | Non -> js_string "Non"
    | Pre -> js_string "Pre"
    | Typ c -> to_js_object "Typ" [| Type.string_of_con c |> js_string |]
    | Named (n, t) -> to_js_object "Name" [| js_string n; typ_js t |]
    | Weak t -> to_js_object "Weak" [| typ_js t |]

  and field_js { Type.lab; typ = t; src = s } =
    to_js_object lab (typ_js t :: src s |> Array.of_list)

  and src ({ Type.depr; track_region; region = r } : Type.src) :
      Js.Unsafe.any list =
    let srcs =
      match srcs_tbl with
      | None -> []
      | Some srcs_tbl -> (
          let open Mo_types in
          match Field_sources.Srcs_map.find_opt track_region srcs_tbl with
          | None -> []
          | Some srcs ->
              List.of_seq @@ Seq.map region_js @@ Source.Region_set.to_seq srcs)
    in
    js_string (Option.value ~default:"" depr) :: region_js r :: srcs

  and bind_js tb =
    let open Type in
    to_js_object tb.var [| typ_js tb.bound |]

  and add_type_annotation (t : Type.typ) (it : Js.Unsafe.any) : Js.Unsafe.any =
    if Cfg.include_types then
      match Cfg.include_type_rep with
      | Arrange.Without_type_rep ->
          to_js_object ":" [| it; Type_pretty.string_of_typ t |> js_string |]
      | Arrange.With_type_rep _ ->
          to_js_object ":"
            [| it; Type_pretty.string_of_typ t |> js_string; typ_js t |]
    else it

  let add_source (at : Source.region) (it : Js.Unsafe.any) : Js.Unsafe.any =
    let open Source in
    if Cfg.include_sources && at <> Source.no_region then
      to_js_object "@" [| syntax_pos_js at.left; syntax_pos_js at.right; it |]
    else it

  let add_trivia (at : Source.region) (it : Js.Unsafe.any) : Js.Unsafe.any =
    match Cfg.include_docs with
    | Some table -> (
        let rec lookup_trivia (line, column) =
          Trivia.PosHashtbl.find_opt table Trivia.{ line; column }
        and find_trivia (parser_pos : Source.region) : Trivia.trivia_info =
          lookup_trivia Source.(parser_pos.left.line, parser_pos.left.column)
          |> Option.get
        in
        match Trivia.doc_comment_of_trivia_info (find_trivia at) with
        | Some s -> to_js_object "*" [| js_string s; it |]
        | None -> it)
    | None -> it

  let id i =
    let open Source in
    add_source i.at (to_js_object "ID" [| js_string i.it |])

  let rec path p =
    let open Source in
    let open Syntax in
    match p.it with
    | IdH i -> to_js_object "IdH" [| id i |]
    | DotH (p, i) -> to_js_object "DotH" [| path p; id i |]

  let lit_js =
    let open Syntax in
    function
    | NullLit -> js_string "NullLit"
    | BoolLit true -> to_js_object "BoolLit" [| js_string "true" |]
    | BoolLit false -> to_js_object "BoolLit" [| js_string "false" |]
    | NatLit n ->
        to_js_object "NatLit" [| js_string (Numerics.Nat.to_pretty_string n) |]
    | Nat8Lit n ->
        to_js_object "Nat8Lit"
          [| js_string (Numerics.Nat8.to_pretty_string n) |]
    | Nat16Lit n ->
        to_js_object "Nat16Lit"
          [| js_string (Numerics.Nat16.to_pretty_string n) |]
    | Nat32Lit n ->
        to_js_object "Nat32Lit"
          [| js_string (Numerics.Nat32.to_pretty_string n) |]
    | Nat64Lit n ->
        to_js_object "Nat64Lit"
          [| js_string (Numerics.Nat64.to_pretty_string n) |]
    | IntLit i ->
        to_js_object "IntLit" [| js_string (Numerics.Int.to_pretty_string i) |]
    | Int8Lit i ->
        to_js_object "Int8Lit"
          [| js_string (Numerics.Int_8.to_pretty_string i) |]
    | Int16Lit i ->
        to_js_object "Int16Lit"
          [| js_string (Numerics.Int_16.to_pretty_string i) |]
    | Int32Lit i ->
        to_js_object "Int32Lit"
          [| js_string (Numerics.Int_32.to_pretty_string i) |]
    | Int64Lit i ->
        to_js_object "Int64Lit"
          [| js_string (Numerics.Int_64.to_pretty_string i) |]
    | FloatLit f ->
        to_js_object "FloatLit"
          [| js_string (Numerics.Float.to_pretty_string f) |]
    | CharLit c -> to_js_object "CharLit" [| js_string (string_of_int c) |]
    | TextLit t -> to_js_object "TextLit" [| js_string t |]
    | BlobLit b -> to_js_object "BlobLit" [| js_string b |]
    | PreLit (s, p) -> to_js_object "PreLit" [| js_string s; prim_js p |]

  let unop_js uo =
    let open Operator in
    js_string
      (match uo with PosOp -> "PosOp" | NegOp -> "NegOp" | NotOp -> "NotOp")

  let binop_js bo =
    let open Operator in
    js_string
      (match bo with
      | AddOp -> "AddOp"
      | SubOp -> "SubOp"
      | MulOp -> "MulOp"
      | DivOp -> "DivOp"
      | ModOp -> "ModOp"
      | AndOp -> "AndOp"
      | OrOp -> "OrOp"
      | XorOp -> "XorOp"
      | ShLOp -> "ShiftLOp"
      | ShROp -> "ShiftROp"
      | RotLOp -> "RotLOp"
      | RotROp -> "RotROp"
      | CatOp -> "CatOp"
      | PowOp -> "PowOp"
      | WAddOp -> "WAddOp"
      | WSubOp -> "WSubOp"
      | WMulOp -> "WMulOp"
      | WPowOp -> "WPowOp")

  let relop_js ro =
    let open Operator in
    js_string
      (match ro with
      | EqOp -> "EqOp"
      | NeqOp -> "NeqOp"
      | LtOp -> "LtOp"
      | GtOp -> "GtOp"
      | LeOp -> "LeOp"
      | GeOp -> "GeOp")

  let rec exp_js e =
    let open Syntax in
    let open Source in
    exp'_js e |> add_type_annotation e.note.note_typ |> add_source e.at

  and exp'_js e =
    let open Syntax in
    let open Source in
    match e.it with
    | HoleE (_, e) -> exp_js !e
    | VarE x -> to_js_object "VarE" [| id x |]
    | LitE l -> to_js_object "LitE" [| lit_js !l |]
    | ActorUrlE e -> to_js_object "ActorUrlE" [| exp_js e |]
    | UnE (ot, uo, e) ->
        to_js_object "UnE"
          [| Type.string_of_typ !ot |> js_string; unop_js uo; exp_js e |]
    | BinE (ot, e1, bo, e2) ->
        to_js_object "BinE"
          [|
            Type.string_of_typ !ot |> js_string;
            exp_js e1;
            binop_js bo;
            exp_js e2;
          |]
    | RelE (ot, e1, ro, e2) ->
        to_js_object "RelE"
          [|
            Type.string_of_typ !ot |> js_string;
            exp_js e1;
            relop_js ro;
            exp_js e2;
          |]
    | ShowE (ot, e) ->
        to_js_object "ShowE" [| Type.string_of_typ !ot |> js_string; exp_js e |]
    | ToCandidE es -> to_js_object "ToCandidE" (exps es |> Array.of_list)
    | FromCandidE e -> to_js_object "FromCandidE" [| exp_js e |]
    | TupE es -> to_js_object "TupE" (exps es |> Array.of_list)
    | ProjE (e, i) ->
        to_js_object "ProjE" [| exp_js e; string_of_int i |> js_string |]
    | ObjBlockE (eo, s, nt, dfs) ->
        to_js_object "ObjBlockE"
          (parenthetical eo
             ([
                obj_sort_js s.it;
                (match nt with
                | None, None -> js_string "_"
                | None, Some t -> syntax_typ_js t
                | Some id, Some t ->
                    to_js_object id.it [| js_string ":"; syntax_typ_js t |]
                | Some id, None -> js_string id.it);
              ]
             @ List.map dec_field_js dfs))
    | ObjE ([], efs) ->
        to_js_object "ObjE" (List.map exp_field_js efs |> Array.of_list)
    | ObjE (bases, efs) ->
        to_js_object "ObjE"
          (exps bases @ [ js_string "with" ] @ List.map exp_field_js efs
          |> Array.of_list)
    | DotE (e, x, _) -> to_js_object "DotE" [| exp_js e; id x |]
    | AssignE (e1, e2) -> to_js_object "AssignE" [| exp_js e1; exp_js e2 |]
    | ArrayE (m, es) ->
        to_js_object "ArrayE" ([ mut_js m ] @ exps es |> Array.of_list)
    | IdxE (e1, e2) -> to_js_object "IdxE" [| exp_js e1; exp_js e2 |]
    | FuncE (x, sp, tp, p, t, sugar, e') ->
        to_js_object "FuncE"
          ([
             js_string (Type.string_of_typ e.note.note_typ);
             shared_pat_js sp;
             js_string x;
           ]
           @ List.map typ_bind_js tp
           @ [
               pat_js p;
               (match t with
               | None -> js_string "_"
               | Some t -> syntax_typ_js t);
               js_string (if sugar then "" else "=");
               exp_js e';
             ]
          |> Array.of_list)
    | CallE (par_opt, e1, ts, (_, e2)) ->
        to_js_object "CallE"
          (parenthetical par_opt ([ exp_js e1 ] @ inst ts @ [ exp_js !e2 ]))
    | BlockE ds -> to_js_object "BlockE" (List.map dec_js ds |> Array.of_list)
    | NotE e -> to_js_object "NotE" [| exp_js e |]
    | AndE (e1, e2) -> to_js_object "AndE" [| exp_js e1; exp_js e2 |]
    | OrE (e1, e2) -> to_js_object "OrE" [| exp_js e1; exp_js e2 |]
    | IfE (e1, e2, e3) ->
        to_js_object "IfE" [| exp_js e1; exp_js e2; exp_js e3 |]
    | SwitchE (e, cs) ->
        to_js_object "SwitchE"
          ([ exp_js e ] @ List.map case_js cs |> Array.of_list)
    | WhileE (e1, e2, _) -> to_js_object "WhileE" [| exp_js e1; exp_js e2 |]
    | LoopE (e1, None, _) -> to_js_object "LoopE" [| exp_js e1 |]
    | LoopE (e1, Some e2, _) -> to_js_object "LoopE" [| exp_js e1; exp_js e2 |]
    | ForE (p, e1, e2, _) ->
        to_js_object "ForE" [| pat_js p; exp_js e1; exp_js e2 |]
    | LabelE (i, t, e) ->
        to_js_object "LabelE" [| id i; syntax_typ_js t; exp_js e |]
    | DebugE e -> to_js_object "DebugE" [| exp_js e |]
    | BreakE (_, Some i, e) -> to_js_object "BreakE" [| id i; exp_js e |]
    | BreakE (_, None, e) -> to_js_object "BreakE" [| exp_js e |]
    | RetE e -> to_js_object "RetE" [| exp_js e |]
    | AsyncE (par_opt, Type.Fut, tb, e) ->
        to_js_object "AsyncE"
          (parenthetical par_opt [ typ_bind_js tb; exp_js e ])
    | AsyncE (None, Type.Cmp, tb, e) ->
        to_js_object "AsyncE*" [| typ_bind_js tb; exp_js e |]
    | AsyncE (Some _, Type.Cmp, tb, e) -> assert false
    | AwaitE (Type.AwaitFut false, e) -> to_js_object "AwaitE" [| exp_js e |]
    | AwaitE (Type.AwaitFut true, e) -> to_js_object "AwaitE?" [| exp_js e |]
    | AwaitE (Type.AwaitCmp, e) -> to_js_object "AwaitE*" [| exp_js e |]
    | AssertE (Runtime, e) -> to_js_object "AssertE" [| exp_js e |]
    | AnnotE (e, t) -> to_js_object "AnnotE" [| exp_js e; syntax_typ_js t |]
    | OptE e -> to_js_object "OptE" [| exp_js e |]
    | DoOptE e -> to_js_object "DoOptE" [| exp_js e |]
    | BangE e -> to_js_object "BangE" [| exp_js e |]
    | TagE (i, e) -> to_js_object "TagE" [| id i; exp_js e |]
    | PrimE p -> to_js_object "PrimE" [| js_string p |]
    | ImportE (f, _fp) -> to_js_object "ImportE" [| js_string f |]
    | ImplicitLibE f -> to_js_object "ImplicitLibE" [| js_string f |]
    | ThrowE e -> to_js_object "ThrowE" [| exp_js e |]
    | TryE (e, cs, None) ->
        to_js_object "TryE"
          ([ exp_js e ] @ List.map catch_js cs |> Array.of_list)
    | TryE (e, cs, Some f) ->
        to_js_object "TryE"
          ([ exp_js e ] @ List.map catch_js cs @ (js_string ";" :: [ exp_js f ])
          |> Array.of_list)
    | IgnoreE e -> to_js_object "IgnoreE" [| exp_js e |]

  and parenthetical po l =
    (if Cfg.include_parenthetical then
       (match po with None -> js_string "_" | Some e -> exp_js e) :: l
     else l)
    |> Array.of_list

  and exps (es : Syntax.exp list) : Js.Unsafe.any list = List.map exp_js es

  and inst i =
    let open Source in
    match i.it with
    | None -> []
    | Some (false, ts) -> List.map syntax_typ_js ts
    | Some (true, ts) -> js_string "system" :: List.map syntax_typ_js ts

  and syntax_typ_js tt =
    let open Source in
    syntax_typ'_js tt.it |> add_type_annotation tt.note |> add_source tt.at

  and typ_item ((id, ty) : Syntax.typ_item) =
    let open Source in
    match id with
    | None -> [ syntax_typ_js ty ]
    | Some { it; _ } -> [ js_string it; syntax_typ_js ty ]

  and syntax_typ'_js =
    let open Syntax in
    let open Source in
    function
    | PathT (p, ts) ->
        to_js_object "PathT"
          ([ path p ] @ List.map syntax_typ_js ts |> Array.of_list)
    | PrimT p -> to_js_object "PrimT" [| js_string p |]
    | ObjT (s, ts) ->
        to_js_object "ObjT"
          ([ obj_sort_js s.it ] @ List.map typ_field_js ts |> Array.of_list)
    | ArrayT (m, t) -> to_js_object "ArrayT" [| mut_js m; syntax_typ_js t |]
    | OptT t -> to_js_object "OptT" [| syntax_typ_js t |]
    | VariantT cts ->
        to_js_object "VariantT" (List.map typ_tag_js cts |> Array.of_list)
    | TupT ts ->
        to_js_object "TupT" (List.concat_map typ_item ts |> Array.of_list)
    | FuncT (s, tbs, at, rt) ->
        to_js_object "FuncT"
          ([ func_sort_js s.it ]
           @ List.map typ_bind_js tbs
           @ [ syntax_typ_js at; syntax_typ_js rt ]
          |> Array.of_list)
    | AsyncT (Type.Fut, t1, t2) ->
        to_js_object "AsyncT" [| syntax_typ_js t1; syntax_typ_js t2 |]
    | AsyncT (Type.Cmp, t1, t2) ->
        to_js_object "AsyncT*" [| syntax_typ_js t1; syntax_typ_js t2 |]
    | AndT (t1, t2) ->
        to_js_object "AndT" [| syntax_typ_js t1; syntax_typ_js t2 |]
    | OrT (t1, t2) ->
        to_js_object "OrT" [| syntax_typ_js t1; syntax_typ_js t2 |]
    | ParT t -> to_js_object "ParT" [| syntax_typ_js t |]
    | NamedT (id, t) ->
        to_js_object "NamedT" [| js_string id.it; syntax_typ_js t |]
    | WeakT t -> to_js_object "WeakT" [| syntax_typ_js t |]

  and typ_field_js tf =
    let open Source in
    typ_field'_js tf.it |> add_source tf.at

  and typ_field'_js =
    let open Syntax in
    function
    | ValF (lab, t, m) ->
        to_js_object "ValF" [| id lab; syntax_typ_js t; mut_js m |]
    | TypF (lab, tbs, t) ->
        to_js_object "TypF"
          ((id lab :: List.map typ_bind_js tbs) @ [ syntax_typ_js t ]
          |> Array.of_list)

  and typ_bind_js tb =
    let open Source in
    typ_bind'_js tb.it |> add_source tb.at

  and typ_bind'_js tb =
    let open Source in
    let open Syntax in
    to_js_object tb.var.it [| syntax_typ_js tb.bound |]

  and typ_tag_js tt =
    let open Source in
    typ_tag'_js tt.it |> add_source tt.at

  and typ_tag'_js tt =
    let open Source in
    let open Syntax in
    to_js_object tt.tag.it [| syntax_typ_js tt.typ |]

  and dec_field_js df =
    let open Source in
    dec_field'_js df.it |> add_source df.at |> add_trivia df.at

  and dec_field'_js df =
    let open Syntax in
    to_js_object "DecField" [| dec_js df.dec; vis_js df.vis; stab_js df.stab |]

  and dec_js d =
    let open Source in
    dec'_js d.it |> add_source d.at |> add_trivia d.at

  and dec'_js =
    let open Syntax in
    let open Source in
    function
    | ExpD e -> to_js_object "ExpD" [| exp_js e |]
    | LetD (p, e, Some f) ->
        to_js_object "LetD" [| pat_js p; exp_js e; exp_js f |]
    | LetD (p, e, None) -> to_js_object "LetD" [| pat_js p; exp_js e |]
    | VarD (x, e) -> to_js_object "VarD" [| id x; exp_js e |]
    | TypD (x, tp, t) ->
        to_js_object "TypD"
          ([ id x ] @ List.map typ_bind_js tp @ [ syntax_typ_js t ]
          |> Array.of_list)
    | ClassD (eo, sp, s, x, tp, p, rt, i, dfs) ->
        to_js_object "ClassD"
          (parenthetical eo
             ((shared_pat_js sp :: id x :: List.map typ_bind_js tp)
             @ [
                 pat_js p;
                 (match rt with
                 | None -> js_string "_"
                 | Some t -> syntax_typ_js t);
                 obj_sort_js s.it;
                 id i;
               ]
             @ List.map dec_field_js dfs))
    | MixinD (p, dfs) ->
       to_js_object "MixinD" ((pat_js p :: List.map dec_field_js dfs) |> Array.of_list)
    | IncludeD (i, e, _) ->
       to_js_object "IncludeD" [| id i; exp_js e |]

  and pat_js p =
    let open Source in
    pat'_js p.it |> add_type_annotation p.note |> add_source p.at

  and pat'_js =
    let open Syntax in
    let open Source in
    function
    | WildP -> js_string "WildP"
    | VarP x -> to_js_object "VarP" [| id x |]
    | TupP ps -> to_js_object "TupP" (List.map pat_js ps |> Array.of_list)
    | ObjP ps -> to_js_object "ObjP" (List.map pat_field_js ps |> Array.of_list)
    | AnnotP (p, t) -> to_js_object "AnnotP" [| pat_js p; syntax_typ_js t |]
    | LitP l -> to_js_object "LitP" [| lit_js !l |]
    | SignP (uo, l) -> to_js_object "SignP" [| unop_js uo; lit_js !l |]
    | OptP p -> to_js_object "OptP" [| pat_js p |]
    | TagP (i, p) -> to_js_object "TagP" [| js_string ("#" ^ i.it); pat_js p |]
    | AltP (p1, p2) -> to_js_object "AltP" [| pat_js p1; pat_js p2 |]
    | ParP p -> to_js_object "ParP" [| pat_js p |]

  and pat_field_js pf =
    let open Source in
    pat_field'_js pf.it |> add_source pf.at

  and pat_field'_js =
    let open Source in
    let open Syntax in
    function
    | ValPF (id, p) -> to_js_object "ValPF" [| js_string id.it; pat_js p |]
    | TypPF id -> to_js_object "TypPF" [| js_string id.it |]

  and shared_pat_js sp =
    let open Source in
    let open Type in
    match sp.it with
    | Local -> js_string "Local"
    | Shared (Write, p) -> to_js_object "Shared" [| pat_js p |]
    | Shared (Query, p) -> to_js_object "Query" [| pat_js p |]
    | Shared (Composite, p) -> to_js_object "Composite" [| pat_js p |]

  and vis_js v =
    let open Syntax in
    let open Source in
    match v.it with
    | Public None -> js_string "Public"
    | Public (Some m) -> to_js_object "Public" [| js_string m |]
    | Private -> js_string "Private"
    | System -> js_string "System"

  and stab_js =
    let open Syntax in
    let open Source in
    function
    | None -> js_string "(Flexible)"
    | Some s -> (
        match s.it with
        | Flexible -> js_string "Flexible"
        | Stable -> js_string "Stable")

  and exp_field_js ef =
    let open Source in
    exp_field'_js ef.it |> add_source ef.at

  and exp_field'_js ef =
    let open Syntax in
    to_js_object "ExpField" [| mut_js ef.mut; id ef.id; exp_js ef.exp |]

  and case_js c =
    let open Source in
    let open Syntax in
    to_js_object "case" [| pat_js c.it.pat; exp_js c.it.exp |]
    |> add_source c.at

  and catch_js c =
    let open Source in
    let open Syntax in
    to_js_object "catch" [| pat_js c.it.pat; exp_js c.it.exp |]

  and prog_js p =
    let open Source in
    to_js_object "Prog" (List.map dec_js p.it |> Array.of_list)
end

include Make (Arrange.Default)
