open Wasm_exts
open Mo_config

open Printf

let name = "moc"
let banner = "Motoko compiler " ^ Source_id.banner
let usage = "Usage: " ^ name ^ " [option] [file ...]"


(* Argument handling *)

type mode = Default | Check | StableCompatible | Compile | Run | Interact | PrintDeps | Explain

let mode = ref Default
let args = ref []
let add_arg source = args := !args @ [source]

let set_mode m () =
  if !mode <> Default && !mode <> m then begin
    eprintf "moc: multiple execution modes specified"; exit 1
  end;
  mode := m

let out_file = ref ""
let link = ref true
let interpret_ir = ref false
let gen_source_map = ref false
let explain_code = ref ""
let stable_types = ref false
let idl = ref false

let valid_metadata_names =
    ["candid:args";
     "candid:service";
     "motoko:stable-types";
     "motoko:compiler"]

let argspec = 
  Args.ai_args
  @ [
  "-c", Arg.Unit (set_mode Compile), " compile programs to WebAssembly";
  "-g", Arg.Set Flags.debug_info, " generate source-level debug information";
  "-r", Arg.Unit (set_mode Run), " interpret programs";
  "-i", Arg.Unit (set_mode Interact), " run interactive REPL (implies -r)";
  "--check", Arg.Unit (set_mode Check), " type-check only";
  "--stable-compatible",
    Arg.Tuple [
      Arg.String (fun fp -> Flags.pre_ref := Some fp);
      Arg.String (fun fp -> Flags.post_ref := Some fp);
      Arg.Unit (set_mode StableCompatible);
      ],
    "<pre> <post> test upgrade compatibility between stable-type signatures <pre> and <post>";
  "--idl", Arg.Unit (fun () ->
    idl := true;
    set_mode Compile ()), (* similar to --stable-types *)
      " compile and emit Candid IDL specification to `.did` file";
  "--print-deps", Arg.Unit (set_mode PrintDeps), " prints the dependencies for a given source file";
  "--print-source-on-error", Arg.Set Flags.print_source_on_error, " prints the source code for error messages";
  "--explain", Arg.String (fun c -> explain_code := c; set_mode Explain ()), " provides a detailed explanation of an error message";
  "-o", Arg.Set_string out_file, "<file>  output file";

  "-v", Arg.Set Flags.verbose, " verbose output";
  "-p", Arg.Set_int Flags.print_depth, "<n>  set print depth";
  "--warn-help",
    Arg.Unit (fun () ->
      let string_of_level = function
        | Flags.Allow -> "A"
        | Flags.Warn -> "W"
        | Flags.Error -> "E"
      in
      List.iter (fun (code, _, desc) ->
        let lvl = Flags.get_warning_level code in
        printf "%s (%s) %s\n" code (string_of_level lvl) desc
      ) Error_codes.warning_codes;
      printf "\nLegend: A - allowed (warning disabled); W - warn (warning enabled); E - error (treated as error)\n";
      exit 0),
    " show available warning codes, current lint level, and descriptions";
  ]
 
  @ Mo_args.warning_args
  @ Args.error_args

  @ [

  "--version",
    Arg.Unit (fun () -> printf "%s\n%!" banner; exit 0), " show version";
  "--map", Arg.Set gen_source_map, " output source map";

  "-t", Arg.Set Flags.trace, " activate tracing in interpreters"]

  @ Args.package_args

  @ [
  "--profile", Arg.Set Flags.profile, " activate profiling counters in interpreters ";
  "--profile-file", Arg.Set_string Flags.profile_file, "<file>  set profiling output file ";
  "--profile-line-prefix", Arg.Set_string Flags.profile_line_prefix, "<string>  prefix each profile line with the given string ";
  "--profile-field",
    Arg.String (fun n -> Flags.(profile_field_names := n :: !profile_field_names)),
      "<field>  profile file includes the given field from the program result ";

  "--public-metadata",
    Arg.String (fun n -> Flags.(public_metadata_names := n :: !public_metadata_names)),
    "<name>  emit icp custom section <name> (" ^
      String.concat " or " valid_metadata_names ^
      ") as `public` (default is `private`)";

  "--omit-metadata",
    Arg.String (fun n -> Flags.(omit_metadata_names := n :: !omit_metadata_names)),
    "<name>  omit icp custom section <name> (" ^
      String.concat " or " valid_metadata_names ^
      ")";

  "-iR", Arg.Set interpret_ir, " interpret the lowered code";
  "-measure-rts-stack", Arg.Set Flags.measure_rts_stack, " measure the maximum rts stack usage (reported by prim \"rts_max_stack\")";
  "-no-await", Arg.Clear Flags.await_lowering, " no await-lowering (with -iR)";
  "-no-async", Arg.Clear Flags.async_lowering, " no async-lowering (with -iR)";

  "-no-link", Arg.Clear link, " do not statically link-in runtime";
  "-no-timer", Arg.Clear Flags.global_timer, " do not create a global timer expiration endpoint";
  "-no-system-api",
    Arg.Unit (fun () -> Flags.(compile_mode := WasmMode)),
      " do not import any system API";
  "-wasi-system-api",
    Arg.Unit (fun () -> Flags.(compile_mode := WASIMode)),
      " use the WASI system API (wasmtime)";
  "-ref-system-api",
  Arg.Unit (fun () -> Flags.(compile_mode := RefMode)),
      " use the reference implementation of the Internet Computer system API (ic-ref-run)";
  (* TODO: bring this back (possibly with flipped default)
           as soon as the multi-value `wasm` library is out.
  "-multi-value", Arg.Set Flags.multi_value, " use multi-value extension";
  "-no-multi-value", Arg.Clear Flags.multi_value, " avoid multi-value extension";
   *)

  "-dp", Arg.Set Flags.dump_parse, " dump parse";
  "-dt", Arg.Set Flags.dump_tc, " dump type-checked AST";
  "-dl", Arg.Set Flags.dump_lowering, " dump intermediate representation";
  "-no-check-ir", Arg.Clear Flags.check_ir, " do not check intermediate code";
  "--release",
  Arg.Unit
    (fun () -> Flags.release_mode := true),
      " ignore debug expressions in source";
  "--debug",
  Arg.Unit
    (fun () -> Flags.release_mode := false),
      " respect debug expressions in source (the default)";
  "--sanity-checks",
  Arg.Unit
    (fun () -> Flags.sanity := true),
  " enable sanity checking in the RTS and generated code";

  "--stable-types",
  Arg.Unit (fun () ->
    stable_types := true;
    set_mode Compile ()), (* similar to --idl *)
      " compile and emit signature of stable types to `.most` file";

  "--stable-regions",
  Arg.Unit (fun () ->
    Flags.use_stable_regions := true),
      " force eager initialization of stable regions metadata (for testing purposes); consumes between 386KiB or 8MiB of additional physical stable memory, depending on current use of ExperimentalStableMemory library";

  "--generational-gc",
  Arg.Unit (fun () -> Flags.gc_strategy := Mo_config.Flags.Generational),
  " use generational GC (only available with legacy/classical persistence)\n\
  \  Deprecated, will be removed in the future. Use --incremental-gc instead.";

  "--incremental-gc",
  Arg.Unit (fun () -> Flags.gc_strategy := Mo_config.Flags.Incremental),
  " use incremental GC (default, works with both enhanced orthogonal persistence and legacy/classical persistence)";

  "--compacting-gc",
  Arg.Unit (fun () -> Flags.gc_strategy := Mo_config.Flags.MarkCompact),
  " use compacting GC (only available with legacy/classical persistence)\n\
  \  Deprecated, will be removed in the future. Use --incremental-gc instead.";

  "--copying-gc",
  Arg.Unit (fun () -> Flags.gc_strategy := Mo_config.Flags.Copying),
  " use copying GC (only available with legacy/classical persistence)\n\
  \  Deprecated, will be removed in the future. Use --incremental-gc instead.";

  "--force-gc",
  Arg.Unit (fun () -> Flags.force_gc := true),
  " disable GC scheduling, always do GC after an update message (for testing)";

  "--experimental-stable-memory",
  Arg.Set_int Flags.experimental_stable_memory,
  " <n> select support for the deprecated `ExperimentalStableMemory.mo` library (n < 0: error, n == 0: warn, n > 0: allow) (default " ^ (Int.to_string Flags.experimental_stable_memory_default) ^ ")";

  "--max-stable-pages",
  Arg.Set_int Flags.max_stable_pages,
  "<n>  set maximum number of pages available for library `ExperimentalStableMemory.mo` (default " ^ (Int.to_string Flags.max_stable_pages_default) ^ ")";

  "--experimental-field-aliasing",
  Arg.Unit (fun () -> Flags.experimental_field_aliasing := true),
  " enable experimental support for aliasing of var fields";

  "--experimental-rtti",
  Arg.Unit (fun () -> Flags.rtti := true),
  " enable experimental support for precise runtime type information (default with enhanced orthogonal persistence)";

  "--rts-stack-pages",
  Arg.Int (fun pages -> Flags.rts_stack_pages := Some pages),
  "<n>  set maximum number of pages available for runtime system stack (default " ^ (Int.to_string Flags.rts_stack_pages_default) ^ ", only available with classical persistence)";

  "--trap-on-call-error",
  Arg.Unit (fun () -> Flags.trap_on_call_error := true),
  " Trap, don't throw an `Error`, when an IC call fails due to destination queue full or freezing threshold is crossed. Emulates behaviour of moc versions < 0.8.0.";

  (* persistence *)
  "--enhanced-orthogonal-persistence",
  Arg.Unit (fun () -> Flags.enhanced_orthogonal_persistence := true;
                      Flags.explicit_enhanced_orthogonal_persistence := true),
  " use enhanced orthogonal persistence (default): Scalable and fast upgrades using a persistent 64-bit main memory. Also, enable upgrade from classical to enhanced orthogonal persistence";

  (* persistence *)
  "--legacy-persistence",
  Arg.Unit (fun () -> Flags.enhanced_orthogonal_persistence := false),
  " use legacy (classical) persistence. This also enables the usage of --copying-gc, --compacting-gc, and --generational-gc. Deprecated in favor of the new enhanced orthogonal persistence, which is default. Legacy persistence will be removed in the future.";

  "-unguarded-enhanced-orthogonal-persistence",
  Arg.Unit (fun () -> Flags.enhanced_orthogonal_persistence := true; Flags.explicit_enhanced_orthogonal_persistence := false),
  Args._UNDOCUMENTED_ "  (internal testing only)";
  ]

  @ Args.persistent_actors_args

  @ [
  "--stabilization-instruction-limit",
  Arg.Int (fun limit -> Flags.(stabilization_instruction_limit := {
    upgrade = Int64.of_int limit;
    update_call = Int64.of_int limit;
  })),
  "<n>  set instruction limit for incremental graph-copy-based stabilization and destabilization (for testing)";

  "--stable-memory-access-limit",
  Arg.Int (fun limit -> Flags.(stable_memory_access_limit := {
    upgrade = Int64.of_int limit;
    update_call = Int64.of_int limit;
  })),
  "<n>  set stable memory access limit for incremental graph-copy-based stabilization and destabilization (for testing)";

  (* optimizations *)
  "-fno-shared-code",
  Arg.Unit (fun () -> Flags.share_code := false),
  " do *not* share low-level utility code: larger code size but decreased cycle consumption (default)";

  "-fshared-code",
  Arg.Unit (fun () -> Flags.share_code := true),
  " do share low-level utility code: smaller code size but increased cycle consumption";

  "--skip-gc-deprecation-warning",
  Arg.Unit (fun () -> Flags.skip_gc_deprecation_warning := true),
  " skip the deprecation warning for the GC strategy flags"

  ]

  @ Args.inclusion_args



let set_out_file files ext =
  if !out_file = "" then begin
    match files with
    | [n] -> out_file := Filename.remove_extension (Filename.basename n) ^ ext
    | ns -> eprintf "moc: no output file specified"; exit 1
  end

(* Main *)

let exit_on_none = function
  | None -> exit 1
  | Some x -> x

let process_files files : unit =
  match !mode with
  | Default ->
    assert false
  | Run ->
    if !interpret_ir
    then exit_on_none (Pipeline.interpret_ir_files files)
    else exit_on_none (Pipeline.run_files files)
  | Interact ->
    printf "%s\n%!" banner;
    exit_on_none (Pipeline.run_files_and_stdin files)
  | Check ->
    Diag.run (Pipeline.check_files files)
  | StableCompatible ->
    begin
      match (!Flags.pre_ref, !Flags.post_ref) with
      | Some pre, Some post ->
        Diag.run (Pipeline.stable_compatible pre post); (* exit 1 on error *)
        exit 0;
      | _ -> assert false
    end
  | Compile ->
    set_out_file files ".wasm";
    let source_map_file = !out_file ^ ".map" in
    let (idl_prog, module_) = Diag.run Pipeline.(compile_files !Flags.compile_mode !link files) in
    let module_ = CustomModule.{ module_ with
      source_mapping_url =
        if !gen_source_map
        then Some (Filename.basename source_map_file)
        else None
    } in

    let oc = open_out !out_file in
    let (source_map, wasm) = CustomModuleEncode.encode module_ in
    output_string oc wasm; close_out oc;

    if !gen_source_map then begin
      let oc_ = open_out source_map_file in
      output_string oc_ source_map; close_out oc_
      end;

    if !idl then begin
      let open Idllib in
      let did_file = Filename.remove_extension !out_file ^ ".did" in
      let oc = open_out did_file in
      let module WithComments = Arrange_idl.Make(struct let trivia = Some idl_prog.Source.note.Syntax.trivia end) in
      let idl_code = WithComments.string_of_prog idl_prog in
      output_string oc idl_code; close_out oc
    end;

    if !stable_types then begin
      let sig_file = Filename.remove_extension !out_file ^ ".most"
      in
      CustomModule.(
        match module_.motoko.stable_types with
        | Some (_, txt) ->
          let oc_ = open_out sig_file in
          output_string oc_ txt; close_out oc_
        | _ -> ())
    end

  | PrintDeps -> begin
     match files with
     | [file] -> Pipeline.print_deps file
     | _ ->
        (eprintf "--print-deps expects exactly one source file as an argument";
         exit 1)
    end
  | Explain ->
     match Error_codes.try_find_explanation !explain_code with
     | Some (_, Some(explanation)) ->
        printf "%s" explanation
     | Some (_, None) ->
        printf "Unfortunately there is no detailed explanation for this error yet"
     | None ->
        printf "%s is not a known error code. Make sure you pass a code format like this: '--explain M0123'" !explain_code

(* Copy relevant flags into the profiler library's (global) settings.
   This indirection affords the profiler library an independence from the (hacky) Flags library.
   See also, this discussion:
   https://github.com/dfinity/motoko/pull/405#issuecomment-503326551
*)
let process_profiler_flags () =
  ProfilerFlags.profile             := !Flags.profile;
  ProfilerFlags.profile_verbose     := !Flags.profile_verbose;
  ProfilerFlags.profile_file        := !Flags.profile_file;
  ProfilerFlags.profile_line_prefix := !Flags.profile_line_prefix;
  ProfilerFlags.profile_field_names := !Flags.profile_field_names;
  ()

let process_metadata_names kind =
  List.iter
    (fun s ->
      if not (List.mem s valid_metadata_names) then
        begin
          eprintf "moc: --%s-metadata argument %s must be one of %s"
            kind
            s
            (String.concat ", " valid_metadata_names);
          exit 1
        end)

let () =
  (*
  Sys.catch_break true; - enable to get stacktrace on interrupt
  (useful for debugging infinite loops)
  *)
  Internal_error.setup_handler ();
  Arg.parse_expand argspec add_arg usage;
  if !mode = Default then mode := (if !args = [] then Interact else Compile);
  Flags.compiled := !mode = Compile;

  if !Flags.warnings_are_errors && (not !Flags.print_warnings)
  then begin
    eprintf "moc: --hide-warnings and -Werror together do not make sense"; exit 1
  end;

  if not !Flags.skip_gc_deprecation_warning 
  then begin
    match !Flags.gc_strategy with
    | Mo_config.Flags.Copying | Mo_config.Flags.MarkCompact | Mo_config.Flags.Generational ->
        eprintf "moc: --%s-gc is deprecated and will be removed in the future. Use --incremental-gc instead.\n" (Flags.gc_strategy_to_str !Flags.gc_strategy); 
      | _ -> ();
  end;

  process_profiler_flags ();
  process_metadata_names "public" !Flags.public_metadata_names;
  process_metadata_names "omit" !Flags.omit_metadata_names;
  try
    match process_files !args with
      (* TODO: Find a better place to gracefully handle the input-dependent linker error *)
    | exception Linking.LinkModule.TooLargeDataSegments error_message ->
      Printf.eprintf "Error: %s" error_message; ()
    | () -> ()
  with
  | Sys_error msg ->
    (* IO error *)
    eprintf "%s\n" msg;
    exit 1
