(* This (or a type like this) could move into the IDL AST *)
type label = Nat of Lib.Uint32.t | Id of string
val pascal_case : string -> string
val escape : string -> string
val escape_num : Lib.Uint32.t -> string
val escape_method : Source.region -> string -> string
val unescape : string -> label
val unescape_hash : string -> Lib.Uint32.t
val unescape_method : string -> string
val needs_candid_quote : string -> bool
