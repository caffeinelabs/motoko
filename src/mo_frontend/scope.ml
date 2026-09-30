module T = Mo_types.Type
module Field_sources = Mo_types.Field_sources
module S = Mo_def.Syntax

(* Scopes *)

type val_kind = Declaration | FieldReference | MixinIncluded | MutableNotAssigned

type val_env = (T.typ * Source.region * val_kind) T.Env.t
(* A value statically known to sit at [path] inside library [lib]. Libraries are
   evaluated once and module fields are immutable, so equal paths denote the
   same value *)
type static_path = { lib : string; path : string list }
(* [lib_aliases] maps a field path of the library to the static path it is bound to *)
type lib_info = {
  lib_typ : T.typ;
  lib_package : string option;
  lib_aliases : (string list * static_path) list;
}
type lib_env = lib_info T.Env.t
type typ_env = T.con T.Env.t
type con_env = T.ConSet.t
type fld_src_env = Field_sources.srcs_map
type mixin_env = mixin_data T.Env.t
and mixin_data =  {
  imports : S.import list;
  need_system : bool;
  arg : S.pat;
  decs : S.dec_field list;
  typ : T.typ;
  trivia : Trivia.triv_table;
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
  T.Env.{ val_env = empty;
    lib_env = empty;
    typ_env = empty;
    con_env = T.ConSet.empty;
    obj_env = empty;
    mixin_env = empty;
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

let lib ?(aliases = []) ~package path typ =
  { empty with lib_env = T.Env.singleton path { lib_typ = typ; lib_package = package; lib_aliases = aliases } }

let mixin f t =
  { empty with mixin_env = T.Env.singleton f t }
