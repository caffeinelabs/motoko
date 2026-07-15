open Mo_types

val suggest_id : string (* desc *) -> string (* id *) -> string list (* ids *) -> string option

val suggest_conversion : Scope.lib_env -> (Type.typ * _ * _ * _) Type.Env.t -> Type.typ -> Type.typ -> string

val module_name_as_url : string -> string

val mo_url_of_path : string -> string option
