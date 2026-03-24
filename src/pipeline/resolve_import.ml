open Mo_def
open Ic
module Traversals = Mo_frontend.Traversals

(*
This module traverses the syntax tree. For each `import` statement, it looks
at the given relative path and tries to resolve it to a full path (where
full means relative to the current working directory, so that source
directories do not creep into the build output). If no file can be found
there, this prints an error message, otherwise it stores the real path
in the second, mutable field of the ImportE statement.

It returns a list of all imported file names.

*)

type filepath = string
type url = string
type envvar = string
type blob = string

type resolved_imports = Syntax.resolved_import Source.phrase list

(* This returns a map from Syntax.resolved_import
   to the location of the first import of that library
*)
module RIM = Map.Make
  (struct
    type t = Syntax.resolved_import
    let compare = compare
  end)

(* The Set variant is used in the pipeline module *)
module S = Set.Make
  (struct
    type t = Syntax.resolved_import
    let compare = compare
  end)


(* a map of type package_map will map each package name to local, non-relative
   filepath e.g.,
   packages("std") = "/Users/home/username/.dfinity-sdk/src/mo-stdlib/0.1.0/"
   packages("foo") = "/Users/home/username/fooPackage/1.2.3/src"
*)
module M = Map.Make(String)
type package_map = filepath M.t

open Syntax
open Source

let err_unrecognized_url msgs at url msg =
  let open Diag in
  add_msg msgs
    (error_message
       at
       "M0006"
       "import"
       (Printf.sprintf "cannot parse import URL \"%s\": %s" url msg))

let err_unrecognized_alias msgs alias principal msg =
  let open Diag in
  add_msg msgs
    (error_message
       no_region
       "M0007"
       "actor-alias"
       (Printf.sprintf "cannot parse principal \"%s\" for actor alias \"%s\": %s" principal alias msg))

let err_actor_import_without_idl_path msgs at =
  let open Diag in
  add_msg msgs
    (error_message
       at
       "M0008"
       "import"
      (Printf.sprintf "cannot import canister urls without --actor-idl param"))

let err_file_does_not_exist' at full_path =
  Diag.error_message
    at
    "M0009"
    "import"
    (Printf.sprintf "file \"%s\" does not exist" full_path)

let err_file_does_not_exist msgs at full_path =
  Diag.add_msg msgs (err_file_does_not_exist' at full_path)

let err_package_not_defined msgs at pkg =
  let open Diag in
  add_msg msgs
    (error_message
       at
       "M0010"
       "import"
       (Printf.sprintf "package \"%s\" not defined" pkg))

let err_alias_not_defined msgs at alias =
  let open Diag in
  add_msg msgs
    (error_message
       at
       "M0011"
       "import"
       (Printf.sprintf "canister alias \"%s\" not defined" alias))

let err_package_file_does_not_exist msgs f pname =
  let open Diag in
  add_msg msgs
    (error_message
       no_region
       "M0012"
       "package"
       (Printf.sprintf "file \"%s\" (for package `%s`) does not exist" f pname))

let err_prim_pkg msgs =
  let open Diag in
  add_msg msgs
    (error_message
       no_region
       "M0013"
       "package" "the \"prim\" package is built-in, and cannot be mapped to a directory")

let append_mo_extension : (string -> bool) -> string -> string =
  fun file_exists f ->
  let file_path = f ^ ".mo" in
  let lib_path = Filename.concat f "lib.mo" in
  if Option.is_some (Lib.String.chop_suffix "/" f) then
    lib_path
  else if file_exists file_path then
    file_path
  else
    lib_path

let resolve_lib_import at full_path append_extension : (string, Diag.message) result =
  let full_path = append_extension Sys.file_exists full_path |> Lib.FilePath.normalise in
  if Sys.file_exists full_path
  then Ok full_path
  else Error (err_file_does_not_exist' at full_path)

let add_lib_import msgs imported ri_ref at lib_path =
  match resolve_lib_import at lib_path.path append_mo_extension with
  | Ok full_path -> begin
      let ri = LibPath {lib_path with path = full_path} in
      ri_ref := ri;
      imported := RIM.add ri at !imported
    end
  | Error err ->
    Diag.add_msg msgs err

let add_idl_import msgs imported ri_ref at full_path envvar_or_bytes =
  if Sys.file_exists full_path
  then begin
    ri_ref := IDLPath (full_path, envvar_or_bytes);
    imported := RIM.add !ri_ref at !imported
  end else
    err_file_does_not_exist msgs at full_path

let add_value_import msgs imported ri_ref at path =
  if !Mo_config.Flags.blob_import_placeholders then begin
    (* When placeholders are enabled, skip file existence check *)
    let ri = ImportedValuePath path in
    ri_ref := ri;
    imported := RIM.add ri at !imported
  end else begin
    let add_no_extension _file_exists f = f in
    match resolve_lib_import at path add_no_extension with
    | Ok full_path -> begin
        let ri = ImportedValuePath full_path in
        ri_ref := ri;
        imported := RIM.add ri at !imported
      end
    | Error err ->
      Diag.add_msg msgs err
  end

let add_prim_import imported ri_ref at =
  ri_ref := PrimPath;
  imported := RIM.add PrimPath at !imported

let in_base base f =
  if base = "."
  then f
  else Filename.concat base f

let resolve_import_string msgs base actor_idl_path aliases packages imported (f, ri_ref, at)  =
  let resolve_ic bytes = match actor_idl_path with
    | None -> err_actor_import_without_idl_path msgs at
    | Some actor_base ->
      let full_path = in_base actor_base (Url.idl_basename_of_blob bytes) in
      add_idl_import msgs imported ri_ref at full_path (Either.Right bytes)
  in
  let resolve_env (envvar, did_path) =
    let full_path = Lib.FilePath.normalise did_path in
    add_idl_import msgs imported ri_ref at full_path (Either.Left envvar)
  in
  match Url.parse f with
  | Ok (Url.Relative path) ->
    (* TODO support importing local .did file *)
    add_lib_import msgs imported ri_ref at
      { path = in_base base path; package = None }
  | Ok (Url.Package (pkg,path)) ->
    begin match M.find_opt pkg packages with
    | Some pkg_path ->
      add_lib_import msgs imported ri_ref at
        { path = in_base pkg_path path; package = Some pkg }
    | None -> err_package_not_defined msgs at pkg
    end
  | Ok (Url.Ic bytes) ->
     if String.length bytes > 29 then
       err_unrecognized_url msgs at f "Principal too long"
     else
     resolve_ic bytes
  | Ok (Url.IcAlias alias) ->
    begin match M.find_opt alias aliases with
    | Some (Either.Right (bytes, None)) -> resolve_ic bytes
    | Some (Either.Right (bytes, Some did_path)) ->
      add_idl_import msgs imported ri_ref at (Lib.FilePath.normalise did_path) (Either.Right bytes)
    | Some (Either.Left (envvar, did_path)) -> resolve_env (envvar, did_path)
    | None -> err_alias_not_defined msgs at alias
    end
  | Ok (Url.FileValue path) ->
    add_value_import msgs imported ri_ref at (in_base base path)
  | Ok Url.Prim ->
    add_prim_import imported ri_ref at
  | Error msg ->
    err_unrecognized_url msgs at f msg

(* Resolve the argument to --package. *)
let resolve_package_url (msgs:Diag.msg_store) (pname:string) (f:url) : filepath =
  if pname = "prim" then (err_prim_pkg msgs ;"") else
  let f = Lib.FilePath.normalise f in
  if Sys.file_exists f
  then f
  else (err_package_file_does_not_exist msgs f pname;"")

(* Resolve the argument to `--actor-alias` and `--actor-env/id-alias`. Check eagerly for well-formedness *)
let resolve_alias_principal (msgs : Diag.msg_store) (alias : string) (f : (envvar * filepath, url * filepath option) Either.t) : (envvar * filepath, blob * filepath option) Either.t =
  let open Either in match f with
  | Left (v, p) ->
    Left (if Lib.Utf8.is_valid v then (v, p) else
      let open Diag in
      (add_msg msgs
        (error_message no_region "M0007" "actor-alias"
          (Printf.sprintf "invalid UTF-8 in environment variable name for actor alias \"%s\"" alias));
      (v, p)))
  | Right (f, fp_opt) ->
    Right ((match Url.decode_principal f with
    | Ok bytes ->
       if String.length bytes > 29 then
         (err_unrecognized_alias msgs alias f "Principal too long"; "")
       else bytes
    | Error msg -> err_unrecognized_alias msgs alias f msg; ""), fp_opt)

let prog_imports (p : prog): (url * resolved_import ref * region) list =
  let res = ref [] in
  let f e = match e.it with
    | ImportE (f, fp) -> res := (f, fp, e.at) ::!res; e
    | _ -> e in
  let _ = ignore (Traversals.over_prog f p) in
  List.rev !res

type actor_idl_path = filepath option
type package_urls = url M.t
type actor_aliases = (envvar * filepath, url * filepath option) Either.t M.t
type aliases = (envvar * filepath, blob * filepath option) Either.t M.t


let resolve_packages : package_urls -> package_map Diag.result = fun purls ->
  Diag.with_message_store (fun msgs -> Some (M.mapi (resolve_package_url msgs) purls))

let resolve_aliases : actor_aliases -> aliases Diag.result = fun alias_principals ->
  Diag.with_message_store (fun msgs -> Some (M.mapi (resolve_alias_principal msgs) alias_principals))

type flags = {
  package_urls : package_urls;
  actor_aliases : actor_aliases;
  actor_idl_path : actor_idl_path;
  include_all_libs : bool;
  }

type resolved_flags = {
  packages : package_map;
  aliases : aliases;
  actor_idl_path : actor_idl_path;
  }


let list_files_recursively : string -> string list =
 fun dir ->
  let rec loop result = function
    | f :: fs when Sys.is_directory f ->
        Sys.readdir f
        |> Array.to_list
        |> List.map (Filename.concat f)
        |> List.append fs
        |> loop result
    | f :: fs -> loop (f :: result) fs
    | [] -> result
  in
  loop [] [ dir ]

let list_files : string -> string list =
  fun source ->
    let all_files = list_files_recursively source in
    List.filter (fun f -> Filename.extension f = ".mo") all_files

let skip_libs_from_package package =
  match !Mo_config.Flags.implicit_package with
  | None -> false
  | Some implicit_package -> package <> implicit_package

let package_imports base packages =
  let base_norm = Lib.FilePath.normalise base in
  let imports = M.fold (fun package url acc ->
    if skip_libs_from_package package then acc else
    let url_norm = Lib.FilePath.normalise url in
    (* Skip when it is a sub-directory, because list_files adds all files recursively *)
    if base_norm = url_norm || Lib.FilePath.relative_to url_norm base_norm <> None then
      acc
    else
      let files = list_files url in
      List.map (fun path ->
          LibPath {package = Some package; path = path}
        ) files::acc)
    packages []
  in
  List.concat imports

let resolve_flags : flags -> resolved_flags Diag.result
  = fun { actor_idl_path; package_urls; actor_aliases; _ } ->
  let open Diag.Syntax in
  let* packages = resolve_packages package_urls in
  let* aliases = resolve_aliases actor_aliases in
  Diag.return { packages; aliases; actor_idl_path }

let resolve
    : flags -> Syntax.prog -> filepath -> resolved_imports Diag.result
  = fun flags p base ->
  let open Diag.Syntax in
  let* { packages; aliases; actor_idl_path } = resolve_flags flags in
  Diag.with_message_store (fun msgs ->
    let base = if Sys.is_directory base then base else Filename.dirname base in
    let imported =
      ref (if flags.include_all_libs
           then (* outside any package, add all available package libraries *)
             (List.fold_right (fun ri rim -> RIM.add ri Source.no_region rim)
               (package_imports base packages) RIM.empty)
           else
             (* consider only the explicitly imported package libraries *)
             RIM.empty)

    in
    List.iter (resolve_import_string msgs base actor_idl_path aliases packages imported)(prog_imports p);
    Some (List.map (fun (rim, at) -> rim @@ at) (RIM.bindings !imported))
  )


let collect_imports (p:prog) base : ((url * url option) list) Diag.result =
  (* TODO unify the code path for resolve and collect_imports *)
  let base = if Sys.is_directory base then base else Filename.dirname base in
  Diag.with_message_store (fun msgs ->
      let imports =
        List.map (fun (f, _, at) ->
            match Url.parse f with
            | Ok (Url.Relative path) -> begin
               match resolve_lib_import at (in_base base path) append_mo_extension with
               | Ok full_path ->
                  (f, Some full_path)
               | Error err ->
                  Diag.add_msg msgs err;
                  (f, None)
              end
            | _ -> (f, None)
          ) (prog_imports p) in
       Some imports
    )
