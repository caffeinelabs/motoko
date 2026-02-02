(** Exception for argument parsing errors.
    Raised instead of calling exit, allowing different handling in moc vs moc.js *)
exception Arg_error of string

(** suppress documentation *)
val _UNDOCUMENTED_ : string -> string

val error_args : (Arg.key * Arg.spec * Arg.doc) list
val package_args : (Arg.key * Arg.spec * Arg.doc) list
val inclusion_args : (Arg.key * Arg.spec * Arg.doc) list
val ai_args : (Arg.key * Arg.spec * Arg.doc) list
val persistent_actors_args : (Arg.key * Arg.spec * Arg.doc) list
