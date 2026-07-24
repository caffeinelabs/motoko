(* (Stable) Signature matching *)

open Mo_types

(* signature matching with multiple error reporting
   c.f. (simpler) Types.match_sig.
*)

(* Docs link for the enhanced multi-migration workflow, used when reporting
   errors against a version that adopts it. *)
val enhanced_migration_link : string

val match_stab_fields : Diag.msg_store -> Source.region -> (* docs link *) string -> Type.mig_lab option ->Type.field list -> (bool * Type.field) list -> unit

val match_stab_sig : Type.stab_sig -> Type.stab_sig -> unit Diag.result
