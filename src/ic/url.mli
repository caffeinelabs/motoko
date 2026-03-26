type parsed =
  | Package of (string * string)
  | Relative of string
  | Ic of string
  | IcAlias of string
  | FileValue of string
  | Prim

val string_of_parsed : parsed -> string
val parse : string -> (parsed, string) result
val idl_basename_of_blob : string -> string
val idl_basename_of_envvar : string -> string

val decode_principal : string -> (string, string) result
val encode_principal : string -> string
