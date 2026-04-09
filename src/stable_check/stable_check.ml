open Mo_def
open Mo_frontend
open Mo_types
open Mo_config

module Resolve_import = Resolve_import

(* Parsing *)

let generic_parse_with ?(recovery=false) mode lexer parser name : _ Diag.result =
  let open Diag.Syntax in
  lexer.Lexing.lex_curr_p <-
    {lexer.Lexing.lex_curr_p with Lexing.pos_fname = name};
  let tokenizer, triv_table = Lexer.tokenizer mode lexer in
  let* mk_syntax =
    try
      Parser_lib.triv_table := triv_table;
      Parsing.parse ~recovery mode (!Flags.error_detail) (parser lexer.Lexing.lex_curr_p) tokenizer lexer
    with Lexer.Error (at, msg) -> Diag.error at "M0002" "syntax" msg
  in
  Diag.return (mk_syntax name)

let parse_with ?(recovery=false) mode lexer parser name : Syntax.prog Diag.result =
  generic_parse_with ~recovery mode lexer parser name

type rel_path = string
type parse_result = (Syntax.prog * rel_path) Diag.result
type no_region_parse_fn = string -> parse_result
type parse_fn = Source.region -> no_region_parse_fn

let parse_string' ?(recovery=false) mode name s : parse_result =
  let open Diag.Syntax in
  let lexer = Lexing.from_string s in
  let parse = Parser.Incremental.parse_prog in
  let* prog = parse_with ~recovery mode lexer parse name in
  Diag.return (prog, name)

let parse_string = parse_string' Lexer.mode
let parse_string_with_recovery = parse_string' ~recovery:true Lexer.mode

let parse_file' ?(recovery=false) mode at filename : parse_result =
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

(* Prelude and internals *)

let builtin_error phase what (msgs : Diag.messages) =
  Printf.eprintf "%s %s failed\n" phase what;
  Diag.print_messages msgs;
  failwith (Printf.sprintf "%s %s failed" phase what)

let check_builtin what src senv0 =
  let lexer = Lexing.from_string src in
  let parse = Parser.Incremental.parse_prog in
  match parse_with Lexer.mode_priv lexer parse what with
  | Error es -> builtin_error "parsing" what es
  | Ok (prog, _ws) ->
    match
      Cons.session ~scope:what (fun () ->
        Typing.infer_prog senv0 None Async_cap.NullCap prog)
    with
    | Error es -> builtin_error "checking" what es
    | Ok ((_t, sscope), _ws) ->
      prog, Scope.adjoin senv0 sscope

let _prelude, initial_stat_env0 =
  check_builtin "prelude" Prelude.prelude Typing.initial_scope

let _internals, initial_stat_env =
  check_builtin "internals" Prelude.internals initial_stat_env0

(* Import resolution *)

let resolve_flags pkg_opt =
  Resolve_import.{
    package_urls = !Flags.package_urls;
    actor_aliases = !Flags.actor_aliases;
    actor_idl_path = !Flags.actor_idl_path;
    include_all_libs = pkg_opt = None && Flags.(!all_libs || !ai_errors || Option.is_some !implicit_package);
  }

let resolve_prog (prog, base) =
  Diag.map
    (fun libs -> (prog, libs))
    (Resolve_import.resolve (resolve_flags None) prog base)

let resolve_progs =
  Diag.traverse resolve_prog

(* Type checking *)

let async_cap_of_prog prog =
  let open Syntax in
  let open Source in
  match (CompUnit.comp_unit_of_prog false prog).it.body.it with
  | ActorClassU _ -> Async_cap.NullCap
  | ActorU _ -> Async_cap.initial_cap()
  | MixinU _ -> Async_cap.initial_cap()
  | ModuleU _ -> assert false
  | ProgU _ ->
    if !Flags.compiled then Async_cap.NullCap
    else Async_cap.initial_cap()

let infer_prog
    ?(enable_type_recovery=false)
    pkg_opt senv async_cap prog : (Type.typ * Scope.scope) Diag.result =
  Cons.session ~scope:prog.Source.note.Syntax.filename (fun () ->
    let open Diag.Syntax in
    let* t_sscope = Typing.infer_prog ~enable_type_recovery pkg_opt senv async_cap prog in
    let* () = Definedness.check_prog prog in
    Diag.return t_sscope)

let check_progs
    ?(enable_type_recovery=false)
    senv progs : (Scope.t list * Scope.t) Diag.result =
  let rec go senv sscopes = function
    | [] -> Diag.return (List.rev sscopes, senv)
    | prog::progs ->
      let open Diag.Syntax in
      let async_cap = async_cap_of_prog prog in
      let* _t, sscope =
        Cons.session ~scope:prog.Source.note.Syntax.filename (fun () ->
          infer_prog ~enable_type_recovery senv None async_cap prog)
      in
      go (Scope.adjoin senv sscope) (sscope :: sscopes) progs
  in
  go senv [] progs

let check_lib senv pkg_opt lib : Scope.scope Diag.result =
  Cons.session ~scope:lib.Source.note.Syntax.filename (fun () ->
    let open Diag.Syntax in
    let* sscope = Typing.check_lib senv pkg_opt lib in
    let* () = Definedness.check_lib lib in
    Diag.return sscope)

let lib_of_prog f prog : Syntax.lib =
  let lib = CompUnit.comp_unit_of_prog true prog in
  { lib with Source.note = { lib.Source.note with Syntax.filename = f } }

(* Prim module *)

let check_prim () : Syntax.lib * Scope.t =
  let lexer = Lexing.from_string (Prelude.prim_module ~timers:!Flags.global_timer) in
  let parse = Parser.Incremental.parse_prog in
  match parse_with Lexer.mode_priv lexer parse "prim" with
  | Error es -> builtin_error "parsing" "prim" es
  | Ok (prog, _ws) ->
    let open Syntax in
    let open Source in
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
    match check_lib initial_stat_env None lib with
    | Error es -> builtin_error "checking" "prim" es
    | Ok (sscope, _ws) ->
      lib, Scope.adjoin initial_stat_env sscope

(* Import chasing *)

let resolved_import_name ri =
  Syntax.(match ri.Source.it with
  | Unresolved -> "/* unresolved */"
  | LibPath { package = _; path }
  | IDLPath (path, _)
  | ImportedValuePath path -> path
  | PrimPath -> "@prim")

type scope_cache = Scope.t Type.Env.t

let chase_imports_cached parsefn senv0 imports scopes_map
    : (Syntax.lib list * Scope.scope * scope_cache) Diag.result =
  let open Diag.Syntax in
  let open Resolve_import.S in
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
      if Type.Env.mem "@prim" !senv.Scope.lib_env then
        Diag.return ()
      else
        let lib, sscope = check_prim () in
        libs := lib :: !libs;
        senv := Scope.adjoin !senv sscope;
        cache := Type.Env.add ri_name sscope !cache;
        Diag.return ()
    | Syntax.Unresolved -> assert false
    | Syntax.(LibPath {path = f; package = lib_pkg_opt}) ->
      if Type.Env.mem f !senv.Scope.lib_env then
        Diag.return ()
      else if mem it !pending then
        Diag.error ri.Source.at "M0003" "import"
          (Printf.sprintf "file %s must not depend on itself" f)
      else begin
        pending := add it !pending;
        let* prog, base = parsefn ri.Source.at f in
        let* () = Static.prog prog in
        let cur_pkg_opt = if lib_pkg_opt <> None then lib_pkg_opt else pkg_opt in
        let* more_imports = Resolve_import.resolve (resolve_flags cur_pkg_opt) prog base in
        let* () = go_set cur_pkg_opt more_imports in
        let lib = lib_of_prog f prog in
        let* sscope = check_lib !senv cur_pkg_opt lib in
        libs := lib :: !libs;
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
      let* prog, idl_scope, actor_opt = Idllib.Pipeline.check_file f in
      if actor_opt = None then
        Diag.error ri.Source.at "M0004" "import"
          (Printf.sprintf "file %s does not define a service" f)
      else
        match Mo_idl.Idl_to_mo.check_prog idl_scope actor_opt with
        | exception Idllib.Exception.UnsupportedCandidFeature error_message ->
          Stdlib.Error [
            Diag.error_message ri.Source.at "M0153" "import"
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
  let* libs, senv, _cache = chase_imports_cached parsefn senv0 imports Type.Env.empty in
  Diag.return (libs, senv)

(* Loading and checking *)

type load_result_cached =
    ( Syntax.lib list
    * (Syntax.prog * string list * Scope.t) list
    * Scope.t
    * scope_cache ) Diag.result

let load_progs_cached
    ?check_actors
    ?(enable_type_recovery=false)
    parsefn files senv scope_cache : load_result_cached =
  let open Diag.Syntax in
  let* parsed = Diag.traverse (parsefn Source.no_region) files in
  let* rs = resolve_progs parsed in
  let progs = List.map fst rs in
  let libs = List.concat_map snd rs in
  let* libs, senv, scope_cache =
    chase_imports_cached parsefn senv libs scope_cache
  in
  let* () = Typing.check_actors ?check_actors senv progs in
  let* sscopes, senv = check_progs ~enable_type_recovery senv progs in
  let prog_result =
    List.map2
      (fun (prog, rims) sscope ->
        prog, List.map resolved_import_name rims, sscope)
      rs sscopes
  in
  Diag.return (libs, prog_result, senv, scope_cache)

let load_progs ?check_actors parsefn files senv =
  let open Diag.Syntax in
  let* libs, rs, senv, _scope_cache =
    load_progs_cached ?check_actors parsefn files senv Type.Env.empty
  in
  let progs = List.map (fun (prog, _, _) -> prog) rs in
  Diag.return (libs, progs, senv)

let check_files ?(enable_recovery=false) files : unit Diag.result =
  let parsefn = if enable_recovery
    then parse_file_with_recovery
    else parse_file
  in
  Diag.map ignore (load_progs parsefn files initial_stat_env)

(* IDL generation *)

let generate_idl files : Idllib.Syntax.prog Diag.result =
  let open Diag.Syntax in
  let* _libs, progs, senv = load_progs ~check_actors:true parse_file files initial_stat_env in
  Diag.return (Mo_idl.Mo_to_idl.prog (progs, senv))

(* Stable compatibility *)

let parse_stab_sig s name =
  let open Diag.Syntax in
  let mode = Lexer.{privileged = false; verification = false} in
  let lexer = Lexing.from_string s in
  let parse = Parser.Incremental.parse_stab_sig in
  let* sig_ = generic_parse_with mode lexer parse name in
  Diag.return sig_

let stable_compatible pre post : unit Diag.result =
  let open Diag.Syntax in
  let* p1 = parse_stab_sig pre "pre" in
  let* p2 = parse_stab_sig post "post" in
  let* s1 =
    Cons.session ~scope:"pre" (fun () ->
      Typing.check_stab_sig initial_stat_env0 p1)
  in
  let* s2 =
    Cons.session ~scope:"post" (fun () ->
      Typing.check_stab_sig initial_stat_env0 p2)
  in
  Stability.match_stab_sig s1 s2
