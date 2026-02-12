open Mo_config
open Common

module Js = Js_of_ocaml.Js
module Sys_js = Js_of_ocaml.Sys_js

let () =
  Sys_js.set_channel_flusher stdout (Buffer.add_string stdout_buffer);
  Sys_js.set_channel_flusher stderr (Buffer.add_string stderr_buffer);
  Flags.ocaml_js := true;
  Flags.check_ir := false;
  Flags.debug_info := false;
  Flags.actor_idl_path := Some "idl/";
  Flags.error_detail := 4; (* unexpected token with expected symbols and examples *)
  Js.export "Motoko"
    (object%js
      val version = js_version
      method setExtraFlags argv = js_set_extra_flags argv
      method saveFile name content = js_save_file name content
      method removeFile name = js_remove_file name
      method renameFile oldpath newpath = js_rename_file oldpath newpath
      method readFile name = js_read_file name
      method readDir path = js_read_dir path
      method addPackage package dir = add_package package dir
      method clearPackage () = clear_package ()
      method setCandidPath path = set_candid_path path
      method setActorAliases entries = set_actor_aliases entries
      method setPublicMetadata entries = set_public_metadata entries
      method setRunStepLimit limit = js_set_run_step_limit limit
      method setTypecheckerCombineSrcs combineSrcs = Flags.typechecker_combine_srcs := combineSrcs
      method setBlobImportPlaceholders placeholders = Flags.blob_import_placeholders := placeholders
      method gcFlags option = gc_flags option
      method run list s = Flags.compiled := false; wrap_output (fun _ -> js_run list s)
      method check s = Flags.compiled := false; js_check s
      method checkWithScopeCache s scopeCache = Flags.compiled := false; js_check_with_scope_cache s scopeCache
      method candid s = Flags.compiled := true; js_candid s
      method stableCompatible pre post = js_stable_compatible pre post
      method compileWasm mode s = Flags.compiled := true; js_compile_wasm mode s
      method parseCandid s = js_parse_candid s
      method parseMotoko enableRecovery s = js_parse_motoko enableRecovery s
      method parseMotokoWithDeps enableRecovery path s = js_parse_motoko_with_deps enableRecovery path s
      (* The method [parseMotokoTyped] is saved for backward compability.
         New versions of tooling should use [parseMotokoTypedWithScopeCache]. *)
      method parseMotokoTyped paths = js_parse_motoko_typed paths
      method parseMotokoTypedWithScopeCache enableRecovery paths scopeCache = js_parse_motoko_typed_with_scope_cache enableRecovery paths scopeCache
      method contextualDotSuggestions scope rawExp = js_contextual_dot_suggestions scope rawExp
      method contextualDotModule rawExp = js_contextual_dot_module rawExp
      method printDeps file = print_deps file
     end);
