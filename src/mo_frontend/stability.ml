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
          "%s does not contain stable variable `%s` — the migration function cannot require it as input\nSee %s"
          (desc mig_lab_opt)
          tf.lab
          link))

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

let incompat_mix_migrations s at =
  Diag.add_msg s
    (Diag.error_message at "M0255" cat
        (Format.asprintf
           "cannot upgrade from an actor using enhanced multi-migration to one that does not — once adopted, it cannot be reverted\nSee %s"
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
    (* The new version dictates which migration mechanism applies, so point at
       the enhanced multi-migration docs when it uses one, and the legacy
       migration-function docs otherwise. *)
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
