open Mo_def
open Mo_config

type filepath = string

module S : Set.S with type elt = Syntax.resolved_import
module RIM : Map.S with type key = Syntax.resolved_import

type actor_idl_path = string option
type package_urls = string Flags.M.t
type actor_aliases = (string * string, string * string option) Either.t Flags.M.t

type resolved_imports = Syntax.resolved_import Source.phrase list

val collect_imports : Syntax.prog -> string -> ((string * string option) list) Diag.result

type flags = {
  package_urls : package_urls;
  actor_aliases : actor_aliases;
  actor_idl_path : actor_idl_path;
  include_all_libs : bool;
  }

type package_map = filepath Flags.M.t
type blob = string
type envvar = string
type aliases = (envvar * filepath, blob * filepath option) Either.t Flags.M.t
type resolved_flags = {
  packages : package_map;
  aliases : aliases;
  actor_idl_path : actor_idl_path;
  }

val resolve_flags : flags -> resolved_flags Diag.result

val resolve : flags -> Syntax.prog -> filepath -> resolved_imports Diag.result

(* Exported for tests *)
val append_mo_extension : (filepath -> bool) -> filepath -> filepath
