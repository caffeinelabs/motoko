open Mo_types.Type

(* For improved warning messages during unused detection:
   - An identifier of a declaration can be renamed, e.g. prefixed by '_'.
   - The field identifier in a field pattern cannot be renamed but can be bound to a new pattern.
   - An identifier introduced via mixin inclusion should not generate unused warnings *)
type val_kind = Declaration | FieldReference | MixinIncluded | MutableNotAssigned
type val_env = (typ * Source.region * val_kind) Env.t
type lib_env = Mo_types.Type.typ Env.t
type typ_env = con Env.t
type con_env = ConSet.t
type fld_src_env = Mo_types.Field_sources.srcs_map
type mixin_env = mixin_data Env.t
and mixin_data =  {
  imports : Mo_def.Syntax.import list;
  arg : Mo_def.Syntax.pat;
  decs : Mo_def.Syntax.dec_field list;
  typ : typ;
}
type obj_env = scope Env.t  (* internal object scopes *)
and scope =
  { val_env : val_env;
    lib_env : lib_env;
    typ_env : typ_env;
    con_env : con_env;
    obj_env : obj_env;
    mixin_env : mixin_env;
    fld_src_env : fld_src_env;
  }
and t = scope

val empty : scope
val adjoin : scope -> scope -> scope

val adjoin_val_env : scope -> val_env -> scope
val lib : string -> typ -> scope
val mixin : string -> mixin_data -> scope
