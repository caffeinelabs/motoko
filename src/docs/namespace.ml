open Source
open Mo_def
module StringMap = Map.Make (String)

type t = {
  types : Xref.t StringMap.t;
  values : (Xref.t * t option) StringMap.t;
}

let rec idents_in_pattern : Syntax.pat -> string list =
 fun pat ->
  match pat.it with
  | Syntax.VarP id -> [ id.it ]
  | Syntax.TupP ps -> List.concat_map idents_in_pattern ps
  | Syntax.ObjP pfs ->
      List.concat_map idents_in_pattern (List.filter_map Syntax.pf_pattern pfs)
  | Syntax.AltP (p, _)
  | Syntax.OptP p
  | Syntax.TagP (_, p)
  | Syntax.AnnotP (p, _)
  | Syntax.ParP p ->
      idents_in_pattern p
  | Syntax.WildP | Syntax.SignP (_, _) | Syntax.LitP _ -> []

let from_module =
  let rec go mk_xref exp_fields =
    List.fold_left
      (fun acc exp_field ->
        match exp_field.it.Syntax.dec.it with
        | Syntax.ExpD _ | Syntax.MixinD _ | Syntax.IncludeD _ -> acc
        | Syntax.LetD
            ( { it = Syntax.VarP id; _ },
              { it = Syntax.ObjBlockE (_, _, _, decs); _ },
              _ ) ->
            let mk_nested x = mk_xref (Xref.XNested (id.it, x)) in
            {
              acc with
              values =
                StringMap.add id.it
                  (mk_xref (Xref.XValue id.it), Some (go mk_nested decs))
                  acc.values;
            }
        | Syntax.LetD (pat, _, _) ->
            let bound_names =
              List.map
                (fun i -> (i, (mk_xref (Xref.XValue i), None)))
                (idents_in_pattern pat)
            in
            {
              acc with
              values =
                StringMap.union
                  (fun k v1 v2 -> Some v1)
                  (StringMap.of_seq (List.to_seq bound_names))
                  acc.values;
            }
        | Syntax.TypD (id, _, _) ->
            {
              acc with
              types = StringMap.add id.it (mk_xref (Xref.XType id.it)) acc.types;
            }
        | Syntax.VarD (id, _) ->
            {
              acc with
              values =
                StringMap.add id.it
                  (mk_xref (Xref.XValue id.it), None)
                  acc.values;
            }
        | Syntax.ClassD (_, _, _, id, _, _, _, _, _) ->
            {
              acc with
              types = StringMap.add id.it (mk_xref (Xref.XType id.it)) acc.types;
              values =
                StringMap.add id.it
                  (mk_xref (Xref.XType id.it), None)
                  acc.values;
            })
      { values = StringMap.empty; types = StringMap.empty }
      exp_fields
  in
  go Fun.id

let from_imports : (string * string) list -> t =
 fun imports ->
  {
    values =
      StringMap.of_seq
        (List.to_seq
           (List.filter_map
              (fun (id, url) ->
                (* TODO: Eventually we might also want to allow
                   types imported from IDL files or actors *)
                match Ic.Url.parse url with
                | Ok (Ic.Url.Relative path) ->
                    Some (id, (Xref.XFile (path, None), None))
                | Ok (Ic.Url.Package (pkg, path)) ->
                    Some
                      ( id,
                        ( Xref.XPackage (pkg, Some (Xref.XFile (path, None))),
                          None ) )
                | _ -> None)
              imports));
    types = StringMap.empty;
  }

let shadow : t -> t -> t =
 fun n1 n2 ->
  {
    values = StringMap.union (fun _ x1 x2 -> Some x2) n1.values n2.values;
    types = StringMap.union (fun _ x1 x2 -> Some x2) n1.types n2.types;
  }

let rec split_path : Syntax.path' -> string list * string = function
  | Syntax.IdH id -> ([], id.Source.it)
  | Syntax.DotH (path, id) ->
      let xs, x = split_path path.Source.it in
      (List.append xs [ x ], id.Source.it)

let lookup_type : t -> Syntax.typ_path -> Xref.t option =
 fun ns path ->
  match split_path path.Source.it with
  | [], id -> StringMap.find_opt id ns.types
  | x :: xs, id -> (
      match StringMap.find_opt x ns.values with
      | None
      | Some (Xref.XValue _, _)
      | Some (Xref.XType _, _)
      | Some (Xref.XClass _, _) ->
          (* None of these let you nest type definitions *)
          None
      | Some ((Xref.XPackage _ as top_xref), _)
      | Some ((Xref.XFile _ as top_xref), _) ->
          (* Until we can verify the full path against an imported
             module's types we're just making one up *)
          let mk_xref =
            List.fold_right
              (fun id f xref -> Xref.XNested (id, f xref))
              xs Fun.id
          in
          Xref.extend top_xref
          |> Option.map (fun mk_top_xref ->
                 mk_top_xref (mk_xref (Xref.XType id)))
      | Some (Xref.XNested _, sub_ns) ->
          let open Lib.Option.Syntax in
          let* sub_ns =
            List.fold_left
              (fun ns id ->
                let* ns in
                let* sub_ns = StringMap.find_opt id ns.values in
                snd sub_ns)
              sub_ns xs
          in
          StringMap.find_opt id sub_ns.types)

let rec format : Format.formatter -> t -> unit =
 fun f { values; types } ->
  let open Format in
  let format_values () =
    pp_open_vbox f 2;
    pp_print_string f "Values:";
    pp_print_cut f ();
    pp_open_vbox f 0;
    StringMap.iter
      (fun k (xref, ns) ->
        pp_open_vbox f 2;
        fprintf f "%s[%s]" k (Xref.to_string xref);
        (match ns with
        | None -> ()
        | Some ns ->
            pp_print_string f ":";
            pp_print_cut f ();
            format f ns);
        pp_close_box f ();
        pp_print_cut f ())
      values;
    pp_close_box f ();
    pp_close_box f ()
  in
  let format_types () =
    pp_open_vbox f 2;
    pp_print_string f "Types:";
    pp_print_cut f ();
    StringMap.iter
      (fun k xref ->
        pp_open_vbox f 0;
        fprintf f "%s[%s]" k (Xref.to_string xref);
        pp_print_cut f ();
        pp_close_box f ())
      types;
    pp_close_box f ()
  in
  pp_open_vbox f 0;
  format_values ();
  pp_print_cut f ();
  format_types ();
  pp_close_box f ()

let to_string : t -> string =
 fun ns ->
  let b = Buffer.create 16 in
  let f = Format.formatter_of_buffer b in
  format f ns;
  Format.pp_print_flush f ();
  Buffer.contents b
