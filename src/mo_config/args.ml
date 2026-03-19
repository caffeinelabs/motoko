(* This module contains some argument parsing that is common between
multiple executables *)
open Exit

(** suppress documentation *)
let _UNDOCUMENTED_ doc = "" (* TODO: enable with developer env var? *)

let string_map ?err inj flag r desc =
  let key_ref = ref "DEADBEEF" in
  let label = Option.value ~default:flag err in
  let open Arg in
  flag,
  Tuple [
    Set_string key_ref;
    String Flags.M.(fun value ->
      let key = !key_ref in
      if mem key !r
      then fail "duplicate %s %s" label key
      else r := add key (inj value) !r
    )
  ],
  desc

let string_map3 ?err inj flag r desc =
  let key_ref = ref "DEADBEEF" in
  let val_ref = ref "DEADBEEF" in
  let label = Option.value ~default:flag err in
  let open Arg in
  flag,
  Tuple [
    Set_string key_ref;
    Set_string val_ref;
    String Flags.M.(fun third ->
      let key = !key_ref in
      if mem key !r
      then fail "duplicate %s %s" label key
      else r := add key (inj (!val_ref, third)) !r
    )
  ],
  desc

(* Everything related to imports, packages, aliases *)
let package_args = [
  string_map Fun.id "--package" Flags.package_urls "<package-name> <package-path> specify a <package-name> <package-path> pair, separated by a space";
  "--actor-idl", Arg.String (fun fp -> Flags.actor_idl_path := Some fp), "<idl-path>   path to actor IDL (Candid) files";
  string_map ~err:"actor alias" (fun p -> Either.Right (p, None)) "--actor-alias" Flags.actor_aliases "<alias> <principal>  actor import alias";
  string_map3 ~err:"actor alias" (fun (p, d) -> Either.Right (p, Some d)) "--actor-id-alias" Flags.actor_aliases "<alias> <principal> <did-path>  actor import alias with explicit IDL path";
  string_map3 ~err:"actor alias" Either.left "--actor-env-alias" Flags.actor_aliases "<alias> <envvar> <did-path>  actor import via environment variable"
  ]

let error_args = [
  "--error-detail", Arg.Set_int Flags.error_detail, "<n>  set error message detail for syntax errors, n in [0..3] (default 2)";
  "--error-recovery", Arg.Set Flags.error_recovery, " report multiple syntax errors";
  "--error-format",     Arg.Symbol (["plain"; "human"; "json"], fun s ->
      Flags.error_format := (match s with
        | "json" -> Flags.Json
        | "human" -> Flags.Human
        | _ -> Flags.Plain)),
    " set error output format"
  (* TODO move --hide-warnings here? *)
  ]

let inclusion_args = [
    (* generic arg inclusion from file *)
  "--args", Arg.Expand Arg.read_arg,
    "<file>  read additional newline separated command line arguments \n\
    \      from <file>";
  "--args0", Arg.Expand Arg.read_arg0,
    "<file>  read additional NUL separated command line arguments from \n\
    \      <file>"
  ]

let ai_args = [
  "--ai-errors", Arg.Set Flags.ai_errors, " emit AI tailored errors";
  "--all-libs", Arg.Set Flags.all_libs, " load all library files from all packages, enabling better diagnostics, e.g. hinting at non-imported items (increases compilation time)";
  "--implicit-package", Arg.String (fun s -> Flags.implicit_package := Some s), _UNDOCUMENTED_ " allow contextual dot and implicits resolution from all modules in the given package"
]

let persistent_actors_args = [
  (* default stability *)
  "--default-persistent-actors",
  Arg.Unit (fun () -> Flags.actors := Flags.DefaultPersistentActors),
  _UNDOCUMENTED_
    " declare every actor (class) as implicitly `persistent`, defaulting actor fields to `stable` (default is --require-persistent-actors). The `persistent` keyword is now optional and redundant.";

  "--require-persistent-actors",
  Arg.Unit (fun () -> Flags.actors := Flags.RequirePersistentActors),
  _UNDOCUMENTED_
    " requires all actors to be declared persistent, defaulting actor fields to `transient` (default). Emit diagnostics to help migrate from non-persistent to `persistent` actors.";

  "--legacy-actors",
  Arg.Unit (fun () -> Flags.actors := Flags.LegacyActors),
  _UNDOCUMENTED_
    " in non-`persistent` actors, silently default actor fields to `transient` (legacy behaviour)";
]
