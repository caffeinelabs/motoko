open Mo_def
open Mo_frontend
open Mo_types
open Mo_values
open Mo_interpreter
open Ir_def
open Ir_interpreter
open Ir_passes
open Mo_config

open Printf

module ResolveImport = Resolve_import

type stat_env = Scope.t
type dyn_env = Interpret.scope
type env = stat_env * dyn_env


(* Diagnostics *)

let phase heading name =
  if !Flags.verbose then printf "-- %s %s:\n%!" heading name

let print_ce =
  Type.ConSet.iter (fun c ->
    let eq, params, typ = Type.pps_of_kind (Cons.kind c) in
    Format.printf "@[<hv 2>type %s%a %s@ %a@]@."
      (Type.string_of_con c)
      params ()
      eq
      typ ()
  )

let print_stat_ve =
  Type.Env.iter (fun x (t, _, _) ->
    let t' = Type.as_immut t in
    Format.printf "@[<hv 2>%s %s :@ %a@]@."
      (if t == t' then "let" else "var") x
      Type.pp_typ t'
  )

let print_dyn_ve scope =
  Value.Env.iter (fun x d ->
    let open Type in
    let (t, _, _) = Env.find x scope.Scope.val_env in
    let t' = as_immut t in
    match normalize t' with
    | Obj (Module, fs, _) ->
      Format.printf "@[<hv 2>%s %s : module {...}@]@."
        (if t == t' then "let" else "var") x
    | _ ->
      Format.printf "@[<hv 2>%s %s :@ %a =@ %a@]@."
        (if t == t' then "let" else "var") x
        Type.pp_typ t'
        (Value.pp_def !Flags.print_depth) (t', d)
  )

let print_scope senv scope dve =
  print_ce scope.Scope.con_env;
  print_dyn_ve senv dve

let print_val _senv t v =
  Format.printf "@[<hv 2>%a :@ %a@]@."
    (Value.pp_val !Flags.print_depth) (t, v)
    Type.pp_typ t


(* Dumping *)

let dump_prog flag prog =
    if !flag then
      Wasm.Sexpr.print 80 (Arrange.prog prog)

let dump_ir flag prog_ir =
    if !flag then
      Wasm.Sexpr.print 80 (Arrange_ir.prog prog_ir)


(* Parsing *)

type rel_path = string

type parse_result = (Syntax.prog * rel_path) Diag.result

type no_region_parse_fn = string -> parse_result
type parse_fn = Source.region -> no_region_parse_fn

let generic_parse_with ?(recovery=false) mode lexer parser name : _ Diag.result =
  phase "Parsing" name;
  let open Diag.Syntax in
  lexer.Lexing.lex_curr_p <-
    {lexer.Lexing.lex_curr_p with Lexing.pos_fname = name};
  (* a back door to enable the `prim` syntax, for our test suite *)
  let tokenizer, triv_table = Lexer.tokenizer mode lexer in
  let* mk_syntax =
    try
      Parser_lib.triv_table := triv_table;
      Parsing.parse ~recovery mode (!Flags.error_detail) (parser lexer.Lexing.lex_curr_p) tokenizer lexer
    with Lexer.Error (at, msg) -> Diag.error at"M0002" "syntax" msg
  in
  let phrase = mk_syntax name in
  Diag.return phrase

let parse_with ?(recovery=false) mode lexer parser name : Syntax.prog Diag.result =
  let open Diag.Syntax in
  let* prog = generic_parse_with ~recovery mode lexer parser name in
  dump_prog Flags.dump_parse prog;
  Diag.return prog

let parse_string' ?(recovery=false) mode name s : parse_result =
  let open Diag.Syntax in
  let lexer = Lexing.from_string s in
  let parse = Parser.Incremental.parse_prog in
  let* prog = parse_with ~recovery mode lexer parse name in
  Diag.return (prog, name)

let parse_string = parse_string' Lexer.mode
let parse_string_with_recovery = parse_string' ~recovery:true Lexer.mode

let parse_file' ?(recovery=false) mode at filename : (Syntax.prog * rel_path) Diag.result =
  let ic, messages = Lib.FilePath.open_in filename in
  Diag.finally (fun () -> close_in ic) (
    let open Diag.Syntax in
    let* _ =
      Diag.traverse_
        (Diag.warn at "M0005" "import")
        messages in
    let lexer = Lexing.from_channel ic in
    let parse = Parser.Incremental.parse_prog in
    let* prog = parse_with ~recovery mode lexer parse filename in
    Diag.return (prog, filename)
  )

let parse_file = parse_file' Lexer.mode
let parse_file_with_recovery = parse_file' ~recovery:true Lexer.mode

let parse_verification_file = parse_file' Lexer.mode_verification

(* Import file name resolution *)

type resolve_result = (Syntax.prog * ResolveImport.resolved_imports) Diag.result

let resolve_flags pkg_opt =
  ResolveImport.{
    package_urls = !Flags.package_urls;
    actor_aliases = !Flags.actor_aliases;
    actor_idl_path = !Flags.actor_idl_path;
    include_all_libs = pkg_opt = None && (!Flags.ai_errors || Option.is_some !Flags.implicit_package);
  }

let resolve_prog (prog, base) : resolve_result =
  Diag.map
    (fun libs -> (prog, libs))
    (ResolveImport.resolve (resolve_flags None) prog base)

let resolve_progs =
  Diag.traverse resolve_prog


(* Printing dependency information *)

let print_deps (file : string) : unit =
  let (prog, _) =  Diag.run (parse_file Source.no_region file) in
  let imports = Diag.run (ResolveImport.collect_imports prog file) in
  List.iter (fun (url, path) ->
      if String.starts_with ~prefix:"blob:" url then () else
      match path with
      | None -> Printf.printf "%s\n" url
      | Some path -> Printf.printf "%s %s\n" url path
    ) imports

(* Checking *)

let async_cap_of_prog prog =
  let open Syntax in
  let open Source in
  match (CompUnit.comp_unit_of_prog false prog).it.body.it with
  | ActorClassU _ -> Async_cap.NullCap
  | ActorU _ -> Async_cap.initial_cap()
  | MixinU _ -> Async_cap.initial_cap()
  | ModuleU _ -> assert false
  | ProgU _ ->
     if !Flags.compiled then
       Async_cap.NullCap
     else
       Async_cap.initial_cap()

let infer_prog
    ?(enable_type_recovery=false)
    pkg_opt
    senv
    async_cap
    prog : (Type.typ * Scope.scope) Diag.result =
  let filename = prog.Source.note.Syntax.filename in
  phase "Checking" filename;
  Cons.session ~scope:filename (fun () ->
    let r = Typing.infer_prog ~enable_type_recovery pkg_opt senv async_cap prog in
    if !Flags.trace && !Flags.verbose then begin
      match r with
      | Ok ((_, scope), _) ->
        print_ce scope.Scope.con_env;
        print_stat_ve scope.Scope.val_env;
        dump_prog Flags.dump_tc prog;
      | Error _ -> ()
    end;
    phase "Definedness" filename;
    let open Diag.Syntax in
    let* t_sscope = r in
    let* () = Definedness.check_prog prog in
    Diag.return t_sscope)

let check_progs
    ?(enable_type_recovery=false)
    senv
    progs : (Scope.t list * Scope.t) Diag.result =
  let rec go senv sscopes = function
    | [] -> Diag.return (List.rev sscopes, senv)
    | prog::progs ->
      let open Diag.Syntax in
      let filename = prog.Source.note.Syntax.filename in
      let async_cap = async_cap_of_prog prog in
      let* _t, sscope =
        Cons.session ~scope:filename (fun () ->
          infer_prog ~enable_type_recovery senv None async_cap prog)
      in
      let senv' = Scope.adjoin senv sscope in
      let sscopes' = sscope :: sscopes in
      go senv' sscopes' progs
  in
  go senv [] progs

let check_lib senv pkg_opt lib : Scope.scope Diag.result =
  let filename = lib.Source.note.Syntax.filename in
  Cons.session ~scope:filename (fun () ->
    phase "Checking" (Filename.basename filename);
    let open Diag.Syntax in
    let* sscope = Typing.check_lib senv pkg_opt lib in
    phase "Definedness" (Filename.basename filename);
    let* () = Definedness.check_lib lib in
    Diag.return sscope)

let lib_of_prog f prog : Syntax.lib  =
  let lib = CompUnit.comp_unit_of_prog true prog in
  { lib with Source.note = { lib.Source.note with Syntax.filename = f } }

(* Prelude and internals *)

let builtin_error phase what (msgs : Diag.messages) =
  Printf.eprintf "%s %s failed\n" phase what;
  Diag.print_messages msgs;
  exit 1

let check_builtin what src senv0 : Syntax.prog * stat_env =
  let lexer = Lexing.from_string src in
  let parse = Parser.Incremental.parse_prog in
  match parse_with Lexer.mode_priv lexer parse what with
  | Error es -> builtin_error "parsing" what es
  | Ok (prog, _ws) ->
    match infer_prog senv0 None Async_cap.NullCap prog with
    | Error es -> builtin_error "checking" what es
    | Ok ((_t, sscope), _ws) ->
      let senv1 = Scope.adjoin senv0 sscope in
      prog, senv1

let prelude, initial_stat_env0 =
  check_builtin "prelude" Prelude.prelude Typing.initial_scope
let internals, initial_stat_env =
  check_builtin "internals" Prelude.internals initial_stat_env0

(* Stable compatibility *)


let parse_stab_sig s name  =
  let open Diag.Syntax in
  let mode = Lexer.{privileged = false; verification = false} in
  let lexer = Lexing.from_string s in
  let parse = Parser.Incremental.parse_stab_sig in
  let* sig_ = generic_parse_with mode lexer parse name in
  Diag.return sig_

let parse_stab_sig_from_file filename : Syntax.stab_sig Diag.result =
  let ic = Stdlib.open_in filename in
  Diag.finally (fun () -> close_in ic) (
    let open Diag.Syntax in
    let mode = Lexer.{privileged = false; verification = false} in
    let lexer = Lexing.from_channel ic in
    let parse = Parser.Incremental.parse_stab_sig in
    let* sig_ = generic_parse_with mode lexer parse filename in
    Diag.return sig_
  )

let stable_compatible pre post : unit Diag.result =
  let open Diag.Syntax in
  let* p1 = parse_stab_sig_from_file pre in
  let* p2 = parse_stab_sig_from_file post in
  let* s1 =
    Cons.session ~scope:p1.Source.note.Syntax.filename (fun () ->
      Typing.check_stab_sig initial_stat_env0 p1)
  in
  let* s2 =
    Cons.session ~scope:p2.Source.note.Syntax.filename (fun () ->
      Typing.check_stab_sig initial_stat_env0 p2)
  in
  Stability.match_stab_sig s1 s2

(* basic sanity checking of emitted stable signatures *)
let validate_stab_sig s : unit Diag.result =
  let open Diag.Syntax in
  let name = "stable-types" in
  Cons.session ~scope:name (fun () ->
    let* p1 = parse_stab_sig s name in
    let* p2 = parse_stab_sig s name in
    let* s1 = Typing.check_stab_sig initial_stat_env0 p1 in
    let* s2 = Typing.check_stab_sig initial_stat_env0 p2 in
    Type.(match s1, s2 with
    | Single s1, Single s2 ->
      (* check we can self-upgrade *)
      Stability.match_stab_sig (Single s1) (Single s2)
    | PrePost (pre1, post1), PrePost (pre2, post2) ->
      (* check we can at least self-upgrade,
         with a possibly different or no migration function *)
      Stability.match_stab_sig (Single post1) (Single post2)
    | _, _ -> assert false))

(* The prim module *)

let prim_name = "prim"

let prim_error phase (msgs : Diag.messages) =
  Printf.eprintf "%s prim failed\n" phase;
  Diag.print_messages msgs;
  exit 1

let check_prim () : Syntax.lib * stat_env =
  let lexer = Lexing.from_string (Prelude.prim_module ~timers:!Flags.global_timer) in
  let parse = Parser.Incremental.parse_prog in
  match parse_with Lexer.mode_priv lexer parse prim_name with
  | Error es -> prim_error "parsing" es
  | Ok (prog, _ws) ->
    let open Syntax in
    let open Source in
    let senv0 = initial_stat_env in
    (* Propagate deprecations *)
    let fs = List.map (fun d ->
      let trivia = Trivia.find_trivia prog.note.trivia d.at in
      let depr = Trivia.deprecated_of_trivia_info trivia in
      {vis = Public depr @@ no_region; dec = d; stab = None} @@ d.at) prog.it
    in
    let body = {it = ModuleU (None, fs); at = no_region; note = empty_typ_note} in
    let lib = {
      it = { imports = []; body };
      at = no_region;
      note = { filename = "@prim"; trivia = Trivia.empty_triv_table }
    } in
    match check_lib senv0 None lib with
    | Error es -> prim_error "checking" es
    | Ok (sscope, _ws) ->
      let senv1 = Scope.adjoin senv0 sscope in
      lib, senv1

(* Imported file loading *)

(*
Loading a file (or string) implies lexing, parsing, resolving imports to
libraries, and typechecking.
The resulting prog is typechecked.
The Typing.scope field in load_result is the accumulated scope.
When we load a declaration (i.e from the REPL), we also care about the type
and the newly added scopes, so these are returned separately.
*)

type scope_cache = Scope.t Type.Env.t

type load_result_cached =
    ( Syntax.lib list
    * (Syntax.prog * string list * Scope.t) list
    * Scope.t
    * scope_cache ) Diag.result

type load_result =
  (Syntax.lib list * Syntax.prog list * Scope.scope) Diag.result

type load_decl_result =
  (Syntax.lib list * Syntax.prog * Scope.scope * Type.typ * Scope.scope) Diag.result

let resolved_import_name ri =
  Syntax.(match ri.Source.it with
  | Unresolved -> "/* unresolved */"
  | LibPath { package = _; path }
  | IDLPath (path, _)
  | ImportedValuePath path -> path
  | PrimPath -> "@prim")

let chase_imports_cached parsefn senv0 imports scopes_map
    : (Syntax.lib list * Scope.scope * scope_cache) Diag.result
  =
  (*
  This function loads and type-checks the files given in `imports`,
  including any further dependencies.

  The resulting `Syntax.libraries` list is in dependency order. To achieve this,
  the function go below does an depth-first traversal of the import DAG.
  * To detected illegal cycles, pending is a set of filenames that we started
    processing, but did not add yet.
  * To avoid duplicates, i.e. load each file at most once, we check the
    senv.
  * We accumulate the resulting libraries in reverse order, for O(1) appending.
  * There is a cache that can be queried to avoid recomputing unchanged dependencies.
  *)

  let open Diag.Syntax in

  let open ResolveImport.S in
  let pending = ref empty in
  let senv = ref senv0 in
  let libs = ref [] in
  let cache = ref scopes_map in

  let rec go_cached pkg_opt ri =
    let ri_name = resolved_import_name ri in
    match Type.Env.find_opt ri_name !cache with
    | None -> Cons.session ~scope:ri_name (fun () -> go pkg_opt ri)
    | Some sscope ->
      senv := Scope.adjoin !senv sscope;
      Diag.return ()
  and go pkg_opt ri =
    let it = ri.Source.it in
    let ri_name = resolved_import_name ri in
    match it with
    | Syntax.PrimPath ->
      (* a bit of a hack, lib_env should key on resolved_import *)
      if Type.Env.mem "@prim" !senv.Scope.lib_env then
        Diag.return ()
      else
        let lib, sscope = check_prim () in
        libs := lib :: !libs; (* NB: Conceptually an append *)
        senv := Scope.adjoin !senv sscope;
        cache := Type.Env.add ri_name sscope !cache;
        Diag.return ()
    | Syntax.Unresolved -> assert false
    | Syntax.(LibPath {path = f; package = lib_pkg_opt}) ->
      if Type.Env.mem f !senv.Scope.lib_env then
        Diag.return ()
      else if mem it !pending then
        Diag.error
          ri.Source.at
          "M0003"
          "import"
          (Printf.sprintf "file %s must not depend on itself" f)
      else begin
        pending := add it !pending;
        let* prog, base = parsefn ri.Source.at f in
        let* () = Static.prog prog in
        let cur_pkg_opt = if lib_pkg_opt <> None then lib_pkg_opt else pkg_opt in
        let* more_imports = ResolveImport.resolve (resolve_flags cur_pkg_opt) prog base in
        let* () = go_set cur_pkg_opt more_imports in
        let lib = lib_of_prog f prog in
        let* sscope = check_lib !senv cur_pkg_opt lib in
        libs := lib :: !libs; (* NB: Conceptually an append *)
        senv := Scope.adjoin !senv sscope;
        cache := Type.Env.add ri_name sscope !cache;
        pending := remove it !pending;
        Diag.return ()
      end
    | Syntax.ImportedValuePath full_path ->
      let sscope = Scope.lib full_path Type.blob in
      senv := Scope.adjoin !senv sscope;
      Diag.return ()
    | Syntax.IDLPath (f, _) ->
      (* TODO: [Idllib.Pipeline.check_file] will perform a similar pipeline,
         going recursively through imports of the IDL path to parse and
         typecheck them. We should extend the cache system to it as well. *)
      let* prog, idl_scope, actor_opt = Idllib.Pipeline.check_file f in
      if actor_opt = None then
        Diag.error
          ri.Source.at
          "M0004"
          "import"
          (Printf.sprintf "file %s does not define a service" f)
      else
        match Mo_idl.Idl_to_mo.check_prog idl_scope actor_opt with
        | exception Idllib.Exception.UnsupportedCandidFeature error_message ->
          Stdlib.Error [
            Diag.error_message
              ri.Source.at
              "M0153"
              "import"
              (Printf.sprintf "file %s uses Candid types without corresponding Motoko type" f);
            error_message ]
        | actor ->
          let sscope = Scope.lib f actor in
          senv := Scope.adjoin !senv sscope;
          cache := Type.Env.add ri_name sscope !cache;
          Diag.return ()
  and go_set pkg_opt todo = Diag.traverse_ (go_cached pkg_opt) todo
  in
  Diag.map (fun () -> List.rev !libs, !senv, !cache) (go_set None imports)

let chase_imports parsefn senv0 imports : (Syntax.lib list * Scope.scope) Diag.result =
  let open Diag.Syntax in
  let cache = Type.Env.empty in
  let* libs, senv, _cache = chase_imports_cached parsefn senv0 imports cache in
  Diag.return (libs, senv)

let load_progs_cached
    ?check_actors
    ?(enable_type_recovery=false)
    parsefn
    files
    senv
    scope_cache : load_result_cached =
  let open Diag.Syntax in
  let* parsed = Diag.traverse (parsefn Source.no_region) files in
  let* rs = resolve_progs parsed in
  let progs = List.map fst rs in
  let libs = List.concat_map snd rs in
  let* libs, senv, scope_cache =
    chase_imports_cached parsefn senv libs scope_cache
  in
  let* () = Typing.check_actors ?check_actors senv progs in
  (* [infer_prog] seems to annotate the AST with types by mutating some of its
     nodes, therefore, we always run the type checker for programs. *)
  let* sscopes, senv = check_progs ~enable_type_recovery senv progs in
  let prog_result =
    List.map2
      (fun (prog, rims) sscope ->
        let rims' = List.map resolved_import_name rims in
        prog, rims', sscope)
      rs
      sscopes
  in
  Diag.return (libs, prog_result, senv, scope_cache)

let load_progs ?check_actors parsefn files senv : load_result =
  let open Diag.Syntax in
  let scope_cache = Type.Env.empty in
  let* libs, rs, senv, _scope_cache =
    load_progs_cached ?check_actors parsefn files senv scope_cache
  in
  let progs = List.map (fun (prog, _immediate_imports, _sscope) -> prog) rs in
  Diag.return (libs, progs, senv)

let load_decl parse_one senv : load_decl_result =
  let open Diag.Syntax in
  let* parsed = parse_one in
  let* prog, libs = resolve_prog parsed in
  let* libs, senv' = chase_imports parse_file senv libs in
  let* t, sscope = infer_prog senv' (Some "<toplevel>") (Async_cap.(AwaitCap top_cap)) prog in
  let senv'' = Scope.adjoin senv' sscope in
  Diag.return (libs, prog, senv'', t, sscope)


(* Interpretation (Source) *)

let interpret_prog denv prog : (Value.value * Interpret.scope) option =
  let open Interpret in
  let filename = prog.Source.note.Syntax.filename in
  phase "Interpreting" filename;
  Cons.session ~scope:filename (fun () ->
    let flags = { trace = !Flags.trace; print_depth = !Flags.print_depth } in
    let result = Interpret.interpret_prog flags denv prog in
    Profiler.process_prog_result result;
    result)

let rec interpret_libs denv libs : Interpret.scope =
  let open Interpret in
  match libs with
  | [] -> denv
  | lib::libs' ->
    let filename = lib.Source.note.Syntax.filename in
    phase "Interpreting" (Filename.basename filename);
    let flags = { trace = !Flags.trace; print_depth = !Flags.print_depth } in
    let dscope =
      Cons.session ~scope:filename (fun () -> interpret_lib flags denv lib)
    in
    let denv' = adjoin_scope denv dscope in
    interpret_libs denv' libs'

let rec interpret_progs denv progs : Interpret.scope option =
  match progs with
  | [] -> Some denv
  | p::ps ->
    match interpret_prog denv p with
    | Some (_v, dscope) ->
      let denv' = Interpret.adjoin_scope denv dscope in
      interpret_progs denv' ps
    | None -> None

let interpret_files (senv0, denv0) files : (Scope.scope * Interpret.scope) option =
  Option.bind
    (Diag.flush_messages (load_progs parse_file files senv0))
    (fun (libs, progs, senv1) ->
      let denv1 = interpret_libs denv0 libs in
      match interpret_progs denv1 progs with
      | None -> None
      | Some denv2 -> Some (senv1, denv2)
    )

let run_builtin prog denv : dyn_env =
  match interpret_prog denv prog with
  | None -> builtin_error "initializing" prog.Source.note.Syntax.filename []
  | Some (_v, dscope) ->
    Interpret.adjoin_scope denv dscope

let initial_dyn_env = run_builtin internals (run_builtin prelude Interpret.empty_scope)

let initial_env = (initial_stat_env, initial_dyn_env)


(* Only checking *)

type check_result = unit Diag.result

let check_files' parsefn files : check_result =
  Diag.map ignore (load_progs parsefn files initial_stat_env)

let check_files ?(enable_recovery=false) files : check_result =
  let parsefn = if enable_recovery
    then parse_file_with_recovery
    else parse_file
  in
  check_files' parsefn files

(* Generate IDL *)

let generate_idl files : Idllib.Syntax.prog Diag.result =
  let open Diag.Syntax in
  let* libs, progs, senv = load_progs ~check_actors:true parse_file files initial_stat_env in
  Diag.return (Mo_idl.Mo_to_idl.prog (progs, senv))

(* Running *)

let run_files files : unit option =
  Option.map ignore (interpret_files initial_env files)

(* Interactively *)

let continuing = ref false

let lexer_stdin buf len =
  let prompt = if !continuing then "  " else "> " in
  printf "%s" prompt; flush_all ();
  continuing := true;
  let rec loop i =
    if i = len then i else
    let ch = input_char stdin in
    Bytes.set buf i ch;
    if ch = '\n' then i + 1 else loop (i + 1)
  in loop 0

let parse_lexer lexer : parse_result =
  let open Lexing in
  if lexer.lex_curr_pos >= lexer.lex_buffer_len - 1 then continuing := false;
  match parse_with Lexer.mode lexer Parser.Incremental.parse_prog_interactive "stdin" with
  | Error es ->
    Lexing.flush_input lexer;
    (* Reset beginning-of-line, too, to sync consecutive positions. *)
    lexer.lex_curr_p <- {lexer.lex_curr_p with pos_bol = 0};
    Error es
  | Ok (prog, ws) -> Ok ((prog, Filename.current_dir_name), ws)

let is_exp dec = match dec.Source.it with Syntax.ExpD _ -> true | _ -> false

let output_scope (senv, _) t v sscope dscope =
  print_scope senv sscope dscope.Interpret.val_env;
  if v <> Value.unit then print_val senv t v

let run_stdin lexer (senv, denv) : env option =
  match Diag.flush_messages (load_decl (parse_lexer lexer) senv) with
  | None ->
    if !Flags.verbose then printf "\n";
    None
  | Some (libs, prog, senv', t, sscope) ->
    let denv' = interpret_libs denv libs in
    match interpret_prog denv' prog with
    | None ->
      if !Flags.verbose then printf "\n";
      None
    | Some (v, dscope) ->
      phase "Finished" "stdin";
      let denv' = Interpret.adjoin_scope denv dscope in
      let env' = (senv', denv') in
      (* TBR: hack *)
      let t', v' =
        if Option.fold ~none:false ~some:is_exp (Lib.List.last_opt prog.Source.it)
        then t, v
        else Type.unit, Value.unit
      in
      output_scope env' t' v' sscope dscope;
      if !Flags.verbose then printf "\n";
      Some env'

let run_stdin_from_file files file : Value.value option =
  let open Lib.Option.Syntax in
  let* (senv, denv) = interpret_files initial_env files in
  let* (libs, prog, senv', t, sscope) =
    Diag.flush_messages (load_decl (parse_file Source.no_region file) senv) in
  let denv' = interpret_libs denv libs in
  let* (v, dscope) = interpret_prog denv' prog in
  print_val senv t v;
  Some v

let run_files_and_stdin files =
  let open Lib.Option.Syntax in
  let lexer = Lexing.from_function lexer_stdin in
  let* env = interpret_files initial_env files in
  let rec loop env = loop (Lib.Option.get (run_stdin lexer env) env) in
  try loop env with End_of_file ->
    printf "\n%!";
    Some ()

(* Desugaring *)

let desugar_unit imports u name : Ir.prog Diag.result =
  match u.Source.it.Syntax.body.Source.it with
  | Syntax.MixinU _ ->
    let at = u.Source.it.Syntax.body.Source.at in
    Diag.error at "M0225" "compile" "A mixin cannot be used as an entry point. It needs to be included in an actor (class)"
  | _ ->
  phase "Desugaring" name;
  let open Lowering.Desugar in
  let prog_ir' : Ir.prog = link_declarations
    (import_prelude prelude @ import_prelude internals @ imports)
    (transform_unit u) in
  dump_ir Flags.dump_lowering prog_ir';
  if !Flags.check_ir
  then Check_ir.check_prog !Flags.verbose "Desugaring" prog_ir';
  Diag.return prog_ir'

(* IR transforms *)

let transform transform_name trans prog name =
  phase transform_name name;
  let prog_ir' : Ir.prog = trans prog in
  dump_ir Flags.dump_lowering prog_ir';
  if !Flags.check_ir
  then Check_ir.check_prog !Flags.verbose transform_name prog_ir';
  prog_ir'

let transform_if transform_name trans flag prog name =
  if flag then transform transform_name trans prog name
  else prog

let await_lowering =
  transform_if "Await Lowering" Await.transform

let async_lowering mode =
  transform_if "Async Lowering" Async.transform

let tailcall_optimization =
  transform_if "Tailcall optimization" Tailcall.transform

let typ_field_translation =
  transform_if "Erase type components" Erase_typ_field.transform

let show_translation =
  transform_if "Translate show" Show.transform

let eq_translation =
  transform_if "Translate polymorphic equality" Eq.transform

let analyze analysis_name analysis prog name =
  phase analysis_name name;
  analysis prog;
  if !Flags.check_ir
  then Check_ir.check_prog !Flags.verbose analysis_name prog

let ir_passes mode prog_ir name =
  (* erase typ components from objects *)
  let prog_ir = typ_field_translation true prog_ir name in
  (* translations that extend the progam and must be done before await/cps conversion *)
  let prog_ir = show_translation true prog_ir name in
  let prog_ir = eq_translation true prog_ir name in
  (* cps conversion and local transformations *)
  let prog_ir = await_lowering !Flags.await_lowering prog_ir name in
  let prog_ir = async_lowering mode !Flags.async_lowering prog_ir name in
  let prog_ir = tailcall_optimization true prog_ir name in
  analyze "constness analysis" Const.analyze prog_ir name;
  prog_ir


(* Compilation *)

let load_as_rts () =
  let rts = match (!Flags.enhanced_orthogonal_persistence, !Flags.sanity, !Flags.gc_strategy) with
    | (true, false, Flags.Incremental) -> Rts.wasm_eop_release
    | (true, true, Flags.Incremental) -> Rts.wasm_eop_debug
    | (false, false, Flags.Copying)
    | (false, false, Flags.MarkCompact)
    | (false, false, Flags.Generational) -> Rts.wasm_non_incremental_release
    | (false, true, Flags.Copying)
    | (false, true, Flags.MarkCompact)
    | (false, true, Flags.Generational) -> Rts.wasm_non_incremental_debug
    | (false, false, Flags.Incremental) -> Rts.wasm_incremental_release
    | (false, true, Flags.Incremental) -> Rts.wasm_incremental_debug
    | _ -> assert false
  in
  Wasm_exts.CustomModuleDecode.decode "rts.wasm" (Lazy.force rts)

type compile_result = (Idllib.Syntax.prog * Wasm_exts.CustomModule.extended_module) Diag.result

let invalid_flag message =
  builtin_error "compile" (Printf.sprintf "Invalid compiler flag combination: %s" message) []

let adjust_flags () =
  if !Flags.enhanced_orthogonal_persistence then
    begin
      (match !Flags.gc_strategy with
      | Flags.Default | Flags.Incremental -> Flags.gc_strategy := Flags.Incremental;
      | Flags.Copying -> invalid_flag "--copying-gc is not supported with --enhanced-orthogonal-persistence"
      | Flags.MarkCompact -> invalid_flag "--compacting-gc is not supported with --enhanced-orthogonal-persistence"
      | Flags.Generational -> invalid_flag "--generational-gc is not supported with --enhanced-orthogonal-persistence");
      (if !Flags.rts_stack_pages <> None then invalid_flag "--rts-stack-pages is not supported with --enhanced-orthogonal-persistence");
      Flags.rtti := true;
    end
  else
    begin
      (if !Flags.gc_strategy = Flags.Default then Flags.gc_strategy := Flags.Copying);
      (if !Flags.rts_stack_pages = None then Flags.rts_stack_pages := Some Flags.rts_stack_pages_default);
      (if !Flags.stabilization_instruction_limit <> Flags.stabilization_instruction_limit_default then
        invalid_flag "--stabilization-instruction-limit is only supported with --enhanced-orthogonal-persistence");
      (if !Flags.stable_memory_access_limit <> Flags.stable_memory_access_limit_default then
         invalid_flag "--stable-memory-access-limit is only supported with --enhanced-orthogonal-persistence")
    end

(* This transforms the flat list of libs (some of which are classes)
   into a list of imported libs and (compiled) classes *)
let rec compile_libs mode libs : Lowering.Desugar.import_declaration =
  let open Source in
  let rec go imports = function
    | [] -> imports
    | l :: libs ->
      let { Syntax.body = cub; _ } = l.it in
      let filename = l.Source.note.Syntax.filename in
      let new_imports =
        Cons.session ~scope:filename (fun () ->
          match cub.it with
          | Syntax.ActorClassU _ ->
            (* Okay to "run" here, as `compile_unit_to_wasm` only fails on Syntax.MixinU *)
            let wasm = Diag.run (compile_unit_to_wasm mode imports l) in
            Lowering.Desugar.import_compiled_class l wasm
          | _ ->
            Lowering.Desugar.import_unit l)
      in
      go (imports @ new_imports) libs
  in go [] libs

and compile_unit mode do_link imports u : Wasm_exts.CustomModule.extended_module Diag.result =
  let open Diag.Syntax in
  let name = u.Source.note.Syntax.filename in
  Cons.session ~scope:name (fun () ->
    let* prog_ir = desugar_unit imports u name in
    let prog_ir = ir_passes mode prog_ir name in
    phase "Compiling" name;
    adjust_flags ();
    let rts = if do_link then Some (load_as_rts ()) else None in
    Diag.return (if !Flags.enhanced_orthogonal_persistence then
      Codegen.Compile_enhanced.compile mode rts prog_ir
    else
      Codegen.Compile_classical.compile mode rts prog_ir))

and compile_unit_to_wasm mode imports (u : Syntax.comp_unit) : string Diag.result =
  let open Diag.Syntax in
  let* wasm_mod = compile_unit mode true imports u in
  let (_source_map, wasm) = Wasm_exts.CustomModuleEncode.encode wasm_mod in
  Diag.return wasm

and compile_progs mode do_link libs progs : Wasm_exts.CustomModule.extended_module Diag.result =
  let imports = compile_libs mode libs in
  let prog = CompUnit.combine_progs progs in
  let u = CompUnit.comp_unit_of_prog false prog in
  compile_unit mode do_link imports u

let compile_files mode do_link files : compile_result =
  let open Diag.Syntax in
  let* libs, progs, senv = load_progs ~check_actors:true parse_file files initial_stat_env in
  let idl = Mo_idl.Mo_to_idl.prog (progs, senv) in
  let* ext_module = compile_progs mode do_link libs progs in
  (* validate any stable type signature, as a sanity check *)
  let* () =
    match Wasm_exts.CustomModule.(ext_module.motoko.stable_types) with
    | Some (_, ss) -> validate_stab_sig ss
    | _ -> Diag.return ()
  in
  let* () =
    if Wasm_exts.CustomModule.(ext_module.wasm_features) <> []
    then Diag.warn Source.no_region "M0191" "compile" (Printf.sprintf "code requires Wasm features %s to execute" (String.concat "," Wasm_exts.CustomModule.(ext_module.wasm_features)))
    else Diag.return ()
  in
  Diag.return (idl, ext_module)


(* Interpretation (IR) *)

(*
   This transforms the flat list of libs into a list of imported units,
   Unlike, `compile_libs`, classes are imported as IR for interpretation,
   not compiled to wasm
*)
let import_libs libs : Lowering.Desugar.import_declaration =
  List.concat_map Lowering.Desugar.import_unit libs

let interpret_ir_progs libs progs =
  let open Diag.Syntax in
  let prog = CompUnit.combine_progs progs in
  let name = prog.Source.note.Syntax.filename in
  let imports = import_libs libs in
  let u = CompUnit.comp_unit_of_prog false prog in
  let* prog_ir = desugar_unit imports u name in
  let prog_ir = ir_passes (!Flags.compile_mode) prog_ir name in
  phase "Interpreting" name;
  let open Interpret_ir in
  let flags = { trace = !Flags.trace; print_depth = !Flags.print_depth } in
  Diag.return (interpret_prog flags prog_ir)

let interpret_ir_files files =
  Diag.flush_messages (Diag.bind
    (load_progs parse_file files initial_stat_env)
    (fun (libs, progs, _) -> interpret_ir_progs libs progs))
