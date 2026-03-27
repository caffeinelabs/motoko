module T = Mo_types.Type
module Field_sources = Mo_types.Field_sources
module S = Mo_def.Syntax

(* Scopes *)

type val_kind = Declaration | FieldReference | MixinIncluded | MutableNotAssigned

type val_env = (T.typ * Source.region * val_kind) T.Env.t
type lib_env = T.typ T.Env.t
type typ_env = T.con T.Env.t
type con_env = T.ConSet.t
type fld_src_env = Field_sources.srcs_map
type mixin_env = mixin_data T.Env.t
and mixin_data =  {
  imports : S.import list;
  arg : S.pat;
  decs : S.dec_field list;
  typ : T.typ;
}
type obj_env = scope T.Env.t  (* internal object scopes *)
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

let empty : scope =
  { val_env = T.Env.empty;
    lib_env = T.Env.empty;
    typ_env = T.Env.empty;
    con_env = T.ConSet.empty;
    obj_env = T.Env.empty;
    mixin_env = T.Env.empty;
    fld_src_env = Field_sources.Srcs_map.empty;
  }

let adjoin scope1 scope2 =
  { val_env = T.Env.adjoin scope1.val_env scope2.val_env;
    lib_env = T.Env.adjoin scope1.lib_env scope2.lib_env;
    typ_env = T.Env.adjoin scope1.typ_env scope2.typ_env;
    con_env = T.ConSet.union scope1.con_env scope2.con_env;
    obj_env = T.Env.adjoin scope1.obj_env scope2.obj_env;
    mixin_env = T.Env.adjoin scope1.mixin_env scope2.mixin_env;
    fld_src_env =
      Field_sources.Srcs_map.adjoin scope1.fld_src_env scope2.fld_src_env;
  }

let adjoin_val_env scope ve = {scope with val_env = T.Env.adjoin scope.val_env ve}

let lib f t =
  { empty with lib_env = T.Env.singleton f t }

let mixin f t =
  { empty with mixin_env = T.Env.singleton f t }
