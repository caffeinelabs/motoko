(* Typed calls to methods with a codec (M0281).

   `encoder`/`decoder` exist for Web2 interop: such a method talks its own
   wire format, and Motoko callers are not its intended clients. Since the
   codecs are not reflected in the method's type, a typed call `a.m(...)`
   still sends Candid (wrong for a `decoder`) and expects a Candid reply
   (wrong for an `encoder`). This is a cheap, best-effort check rather than
   a typing rule: it only recognises receivers whose type is that of an
   actor or actor class from the same compilation unit (self-calls and
   instances), compared structurally. Calls through a `.did` import cannot
   reach such methods at all (they are left out of the interface), and a
   hand-written actor type goes undetected. Raw calls are fine. *)

open Mo_def
open Mo_types
open Syntax
open Source
module T = Type

let check_decs msgs (decs : dec list) =
  let actors = ref [] in
  let add t dfs =
    match codec_methods dfs with
    | [] -> ()
    | ms -> actors := (T.normalize t, ms) :: !actors in
  let scan ds = List.iter (fun (d : dec) -> match d.it with
    | ClassD (_, _, { it = T.Actor; _ }, id, [], _, _, _, dfs) ->
      Option.iter (fun c -> add (T.Con (c, [])) dfs) id.note
    | _ -> ()) ds in
  let collect (e : exp) = (match e.it with
    | ObjBlockE (_, { it = sort; _ }, _, dfs) ->
      if sort = T.Actor then add e.note.note_typ dfs;
      scan (List.map (fun (df : dec_field) -> df.it.dec) dfs)
    | BlockE ds -> scan ds
    | _ -> ()); e in
  let over f = List.iter (fun d -> ignore (Traversals.over_dec f d)) decs in
  scan decs;
  over collect;
  if !actors <> [] then
    over (fun e -> (match e.it with
      | CallE (_, { it = DotE (recv, m, _); _ }, _, _) ->
        let t = T.normalize recv.note.note_typ in
        (match t with
         | T.Obj (T.Actor, _, _) ->
           (match List.find_opt (fun (t', ms) -> T.eq t t' && List.mem_assoc m.it ms) !actors with
            | Some (_, ms) ->
              let has_decoder, has_encoder = List.assoc m.it ms in
              let warn what =
                Diag.add_msg msgs (Diag.warning_message e.at "M0281" "type"
                  (Printf.sprintf "this call %s; use a raw call instead" what)) in
              if has_decoder then
                warn (Printf.sprintf "sends Candid to method %s, whose `decoder` expects its own wire format" m.it);
              if has_encoder then
                warn (Printf.sprintf "expects Candid back from method %s, whose `encoder` replies in its own wire format" m.it)
            | None -> ())
         | _ -> ())
      | _ -> ()); e)

let check_prog prog =
  Diag.with_message_store (fun msgs -> check_decs msgs prog.it; Some ())

let check_lib lib =
  Diag.with_message_store (fun msgs ->
    let (imp_ds, ds) = CompUnit.decs_of_lib lib in
    check_decs msgs (imp_ds @ ds); Some ())
