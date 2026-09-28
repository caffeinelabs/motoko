open Mo_types
open Source
open Type

module Pretty = Type.MakePretty(Type.ElideStampsAndHashes)

let migration_link =
  "https://docs.internetcomputer.org/languages/motoko/fundamentals/actors/compatibility/#explicit-migration-using-a-migration-function"

let enhanced_migration_link =
  "https://docs.internetcomputer.org/languages/motoko/fundamentals/actors/enhanced-multi-migration/"

(* Signature matching *)

let cat = "Compatibility"

(* signature matching with multiple error reporting
   c.f. (simpler) Types.match_sig.
*)

let display_typ = Lib.Format.display Pretty.pp_typ

let display_typ_expand = Lib.Format.display Pretty.pp_typ_expand

let desc mig_lab_opt =
  match mig_lab_opt with
  | None -> "the previous version"
  | Some mig_lab -> "version `" ^ mig_lab ^ "`"

(* the demanding side of the --stable-baseline boundary *)
let em_desc mig_lab_opt =
  match mig_lab_opt with
  | None -> "initial actor"
  | Some mig_lab -> "upgrade resuming after migration `" ^ mig_lab ^ "`"

(* FUTURE: we could perhaps use tf.src.region to better locate the errors below *)
let error_discard s at link mig_lap_opt tf =
  Diag.add_msg s
    (Diag.error_message at "M0169" cat
       (Format.asprintf
          "stable variable `%s` of %s cannot be implicitly discarded. The variable can only be dropped by an explicit migration function.\nSee %s"
          tf.lab
          (desc mig_lap_opt)
          link))

let error_sub s at link mig_lab_opt tf1 tf2 explanation =
  Diag.add_msg s
    (Diag.error_message at "M0170" cat
      (Format.asprintf
         "stable variable `%s` is not compatible with %s.\nPrevious type%a\n is not a subtype of%a\n because %s.\nWrite an explicit migration function to convert it.\nSee %s"
         tf1.lab
        (desc mig_lab_opt)
        display_typ_expand tf1.typ
        display_typ_expand tf2.typ
        (Pretty.string_of_explanation explanation)
        link
))

let error_stable_sub s at link mig_lab_opt tf1 tf2 explanation =
  Diag.add_msg s
    (Diag.error_message at "M0216" cat
      (Format.asprintf
         "stable variable `%s` implicitly drops data of %s.\nPrevious type%a\n is not a stable subtype of%a\n because %s.\nThe data can only be dropped by an explicit migration function.\nSee %s"
         tf1.lab
        (desc mig_lab_opt)
        display_typ_expand tf1.typ
        display_typ_expand tf2.typ
        (Pretty.string_of_explanation explanation)
        link))

let error_required s at link mig_lab_opt tf =
  Diag.add_msg s
    (Diag.error_message at "M0263" cat
       (Format.asprintf
          "%s does not contain the stable variable `%s`. The migration function cannot require this variable as input.\nSee %s"
          (desc mig_lab_opt)
          tf.lab
          link))

(**
  The demand of the migration chain alone at the resume point named by [mig_lab_opt]:
  inputs some migration consumes that no earlier migration produces, i.e. pre over an empty post.

  Any malformed entry (already diagnosed M0201-M0203) yields no demand:
  a hole-y chain is not a valid pre, and empty Multi is invalid. *)
let chain_input_fields mig_lab_opt chain =
  if chain = [] then [] else
  let exception Malformed in
  try
    let chain_fields = chain |> List.map (fun (file, _, typ) ->
      if Type.is_migration typ
      then {lab = Type.migration_lab_of_filename file; typ; src = empty_src}
      else raise Malformed)
    in
    List.map snd (pre mig_lab_opt (Multi {chain = chain_fields; post = []}))
  with Malformed -> []

(* an input of the migration chain itself: no new migration file can sort
   before its consumer, so the previous version must provide it — no hint *)
let em_error_chain_input s at tf =
  Diag.add_msg s
    (Diag.error_message at "M0267" "type"
       (Format.asprintf
          "the migration chain requires field `%s` of type%a as input; the previous version must provide it.\nSee %s"
          tf.lab display_typ tf.typ enhanced_migration_link))

let em_error_required s at mig_lab_opt tf =
  Diag.add_msg s
    (Diag.error_message at "M0267" "type"
       (Format.asprintf
          "%s requires field `%s` of type%a; not found in the previous version — write a migration that produces it.\nSee %s"
          (em_desc mig_lab_opt) tf.lab display_typ tf.typ enhanced_migration_link))

(*
   - Mutability of stable fields can be changed because they are never aliased.
   - Stable fields cannot be dropped.
   - Lossy promotion to any or dropping record fields is rejected (stricter than subtyping to prevent data loss).
 *)

let match_stab_fields s at link mig_lab_opt tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  let cmp tf1 (_, tf2) = compare_field tf1 tf2 in
  Lib.List.align cmp tfs1 tfs2
    |> Seq.iter (function
      (* no dropped fields *)
      | Lib.This tf1 ->
        error_discard s at link mig_lab_opt tf1
      (* new field ok *)
      | Lib.That (required, tf) ->
        if required then error_required s at link mig_lab_opt tf
      | Lib.Both (tf1, (_, tf2)) ->
        let context = [StableVariable tf2.lab] in
        begin
          match Type.sub_explained context (as_immut tf1.typ) (as_immut tf2.typ) with
          | Incompatible explanation -> error_sub s at link mig_lab_opt tf1 tf2 explanation
          | Compatible ->
             match Type.stable_sub_explained context (as_immut tf1.typ) (as_immut tf2.typ) with
             | Incompatible explanation -> error_stable_sub s at link mig_lab_opt tf1 tf2 explanation
             | Compatible -> ()
        end)

(**
  EM counterpart of match_stab_fields for the --stable-baseline boundary:
  EM fields have no initializers, so all are "required" (must be explained by the baseline).

  Compares the deployed fields (tfs1) with the fields demanded at the chain's resume point (tfs2, named by mig_lab_opt);
  a missing demanded field is always an error, never optional.

  `chain_input` (see chain_input_fields) holds the fields no new migration file can fix,
  so their error carries no produce-a-migration hint. *)
let match_stab_em_fields s at mig_lab_opt chain_input tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  let field_at tf =
    let r = tf.src.region in
    if r <> Source.no_region then r else at
  in
  Lib.List.align compare_field tfs1 tfs2
  |> Seq.iter (function
    (* no dropped fields *)
    | Lib.This tf1 ->
      error_discard s at enhanced_migration_link mig_lab_opt tf1
    | Lib.That tf ->
      if lookup_val_field_opt tf.lab chain_input <> None
      then em_error_chain_input s (field_at tf) tf
      else em_error_required s (field_at tf) mig_lab_opt tf
    | Lib.Both (tf1, tf2) ->
      let context = [StableVariable tf2.lab] in
      begin
        match Type.sub_explained context (as_immut tf1.typ) (as_immut tf2.typ) with
        | Incompatible explanation -> error_sub s (field_at tf2) enhanced_migration_link mig_lab_opt tf1 tf2 explanation
        | Compatible ->
           match Type.stable_sub_explained context (as_immut tf1.typ) (as_immut tf2.typ) with
           | Incompatible explanation -> error_stable_sub s (field_at tf2) enhanced_migration_link mig_lab_opt tf1 tf2 explanation
           | Compatible -> ()
      end)

let incompat_mix_migrations s at =
  Diag.add_msg s
    (Diag.error_message at "M0255" cat
        (Format.asprintf
           "cannot upgrade from an actor using enhanced migration to an actor not using enhanced migration.\nSee %s"
           enhanced_migration_link))

let match_stab_sig sig1 sig2 : unit Diag.result =
  match (sig1, sig2) with
  (* Applying regular/old migration on top of a program that
  already uses multi-migration is disallowed. *)
  | Multi _,  (PrePost _ |  Single _) ->
    assert (not (Type.match_stab_sig sig1 sig2));
    Diag.with_message_store (fun s ->
      incompat_mix_migrations s no_region;
      None)
  | _ ->
    let tfs1, mig_lab_opt = post sig1 in
    let tfs2 = pre mig_lab_opt sig2 in
    let link = match sig2 with
      | Multi _ -> enhanced_migration_link
      | Single _ | PrePost _ -> migration_link
    in
    (* Assume that tfs1 and tfs2 are sorted. *)
    let res = Diag.with_message_store (fun s ->
      Some (match_stab_fields s no_region link None tfs1 tfs2))
    in
    (* cross check with simpler definition *)
    match res with
    | Ok _ ->
      assert (Type.match_stab_sig sig1 sig2);
      res
    | Error _ ->
      assert (not (Type.match_stab_sig sig1 sig2));
      res
