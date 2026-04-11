open Mo_types

open Syntax

let (@~) it at = Source.annotate (Const, None) it at

(* Compilation unit detection *)

let is_actor_def e =
  let open Source in
  match e.it with
  | AwaitE (Type.AwaitFut false, { it = AsyncE (_, Type.Fut, _, {it = ObjBlockE (_eo, {it = Type.Actor; _}, _t, _fields); _ }); _ }) -> true
  | _ -> false

let as_actor_def e =
  let open Source in
  match e.it with
  | AwaitE (Type.AwaitFut false, { it = AsyncE (_, Type.Fut, _, {it = ObjBlockE (eo, {it = Type.Actor; note = persistence; _},  _t, fields); note; at }); _ }) ->
    persistence, eo, fields, note, at
  | _ -> assert false

let is_module_def e =
  let open Source in
  match e.it with
  | ObjBlockE (_, { it = Type.Module; _}, _, _) -> true
  | _ -> false

(* Happens after parsing, before type checking *)
let comp_unit_of_prog as_lib (prog : prog) : comp_unit =
  let open Source in
  let f = prog.note in

  let finish imports u =
    { it = { imports = List.rev imports; body = u }; note = f; at = no_region } in
  let prog_typ_note = { empty_typ_note with note_typ = Type.unit } in

  let rec go imports ds : comp_unit =
    match ds with
    (* imports *)
    | {it = LetD (p, ({it = ImportE (url, ri); _} as e), None); _} :: ds' ->
      let i : import = { it = (p, url, ri); note = e.note.note_typ; at = e.at } in
      go (i :: imports) ds'

    (* terminal expressions *)
    | [{it = ExpD ({it = ObjBlockE (_eo, {it = Type.Module; _}, _t, fields); _} as e); _}] when as_lib ->
      finish imports { it = ModuleU (None, fields); note = e.note; at = e.at }
    | [{it = ExpD e; _}] when is_actor_def e ->
      let persistence, eo, fields, note, at = as_actor_def e in
      finish imports { it = ActorU (persistence, eo, None, fields); note; at }
    | [{it = ClassD (eo, sp, {it = Type.Actor; note = persistence; _}, tid, tbs, p, typ_ann, self_id, fields); _} as d] ->
      assert (List.length tbs > 0);
      finish imports { it = ActorClassU (persistence, eo, sp, tid, tbs, p, typ_ann, self_id, fields); note = d.note; at = d.at }
    | [{it = MixinD (args, fields); _} as d] ->
      finish imports { it = MixinU (args, fields); note = d.note; at = d.at }
    (* let-bound terminal expressions *)
    | [{it = LetD ({it = VarP i1; _}, ({it = ObjBlockE (_eo, {it = Type.Module; _}, _t, fields); _} as e), _); _}] when as_lib ->
      finish imports { it = ModuleU (Some i1, fields); note = e.note; at = e.at }
    | [{it = LetD ({it = VarP i1; _}, e, _); _}] when is_actor_def e ->
      let persistence, eo, fields, note, at = as_actor_def e in
      finish imports { it = ActorU (persistence, eo, Some i1, fields); note; at }

    (* Everything else is a program *)
    | ds' ->
      if as_lib
      then
        (* Deprecated syntax, see Typing.check_lib *)
        (* Propagate deprecations *)
        let fs = List.map (fun d ->
          let trivia = Trivia.find_trivia prog.note.trivia d.at in
          let depr = Trivia.deprecated_of_trivia_info trivia in
          {vis = Public depr @@ no_region; dec = d; stab = None} @@ d.at) ds'
        in
        finish imports {it = ModuleU (None, fs); at = no_region; note = empty_typ_note}
      else finish imports { it = ProgU ds; note = prog_typ_note; at = no_region }
  in
  go [] prog.it


(* Lib as decs *)
let obj_decs obj_sort at note id_opt fields =
  let open Source in
  let persistence at = { it = false; at; note = [] }  in
  match id_opt with
  | None -> [
    { it = ExpD {
        it = ObjBlockE ( None, { it = obj_sort; at; note = persistence no_region }, (None, None), fields);
        at;
        note };
      at; note }]
  | Some id -> [
    { it = LetD (
        { it = VarP id; at; note = note.note_typ },
        { it = ObjBlockE ( None, { it = obj_sort; at; note = persistence no_region }, (None, None), fields);
          at; note; },
        None);
      at; note
    };
    { it = ExpD { it = VarE (id.it @~ id.at); at; note };
      at; note }
    ]

(* To enable uniform definedness checking, typechecking and interpretation,
   present the unit as a list of declarations.
*)
let decs_of_lib (cu : comp_unit) =
  let open Source in
  let { imports; body = cub; _ } = cu.it in
  let import_decs = List.map (fun {it = (pat, fp, ri); at; note} ->
    let note = { empty_typ_note with note_typ = note } in
    { it = LetD (
               pat,
               { it = ImportE (fp, ri); at; note },
               None);
      at;
      note }) imports
  in
  import_decs,
  match cub.it with
  | ModuleU (id_opt, fields) ->
    obj_decs Type.Module cub.at cub.note id_opt fields
  | ActorClassU (persistence, eo, csp, i, tbs, p, t, i', efs) ->
    [{ it = ClassD (eo, csp, { it = Type.Actor; at = no_region; note = persistence}, i, tbs, p, t, i', efs);
       at = cub.at;
       note = cub.note;}];
  | MixinU (pat, fields) ->
    [{ it = MixinD (pat, fields); at = no_region; note = cub.note }]
  | ProgU _
  | ActorU _ ->
    assert false

(* a hack to support compiling multiple files *)
let combine_progs (progs : prog list) : prog =
  let open Source in
  if progs = []
  then
    { it = [];
      at = no_region;
      note = { filename = "empty"; trivia = Trivia.empty_triv_table }
    }
  else
    { it = List.concat_map (fun p -> p.it) progs;
      at = (Lib.List.last progs).at;
      note = (Lib.List.last progs).note
    }
