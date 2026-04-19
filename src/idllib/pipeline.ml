open Printf
open Source

(* Diagnostics *)

let phase heading name =
  if !Flags.verbose then printf "-- %s %s:\n%!" heading name

let error at cat text =
  Error [Diag.error_message at "" cat text]

let print_stat_te =
  Typing.Env.iter (fun x t ->
    printf "%s %s = %s\n"
      "type" x (Arrange_idl.string_of_typ t)
  )

let dump_prog flag prog =
    if flag then
      Wasm.Sexpr.print 80 (Arrange_idl.prog prog)

(* Parsing *)

type rel_path = string

type parse_result = (Syntax.prog * rel_path) Diag.result

let parse_with lexer parser name =
  try
    phase "Parsing" name;
    lexer.Lexing.lex_curr_p <-
      {lexer.Lexing.lex_curr_p with Lexing.pos_fname = name};
    let prog = parser Lexer.token lexer name in
    Ok prog
  with
    | ParseError (at, msg) ->
      error at "syntax" msg
    | Parser.Error ->
      error (Lexer.region lexer) "syntax" "unexpected token"

let parse_file filename : parse_result =
  let ic = open_in filename in
  let lexer = Lexing.from_channel ic in
  let parser = Parser.parse_prog in
  let name = Filename.basename filename in
  let result = parse_with lexer parser name in
  close_in ic;
  match result with
  | Ok prog ->
     dump_prog !Flags.dump_parse prog;
     Diag.return (prog, filename)
  | Error e -> Error e

let parse_string s : parse_result =
  let lexer = Lexing.from_string s in
  let parser = Parser.parse_prog in
  let result = parse_with lexer parser "source1" in
  match result with
  | Ok prog -> Diag.return (prog, "source2")
  | Error e -> Error e

let parse_file filename : parse_result =
  try parse_file filename
  with Sys_error s ->
    error no_region "file" (sprintf "cannot open \"%s\"" filename)

(* Type checking *)

let check_prog senv prog
  : (Typing.scope * Syntax.typ option) Diag.result =
  phase "Checking" prog.note.Syntax.filename;
  let r = Typing.check_prog senv prog in
  (match r with
   | Ok ((scope, _), _) ->
      if !Flags.verbose then print_stat_te scope;
   | Error _ -> ());
  r

(* Imported file loading *)

type load_result = (Syntax.prog * Typing.scope * Syntax.typ option) Diag.result

module LibEnv = Env.Make(String)

let merge_env imports init_env lib_env =
  let disjoint_union env1 env2 : Typing.typ_env Diag.result =
    try Diag.return (Typing.Env.union (fun k v1 v2 ->
        (* TODO Add proper type equivalence check for env *)
        if v1 = v2 then Some v1 else raise (Typing.Env.Clash k)
      ) env1 env2)
    with Typing.Env.Clash k ->
      error no_region "import" (sprintf "conflict type definition for %s" k) in
  let env_list = List.map (fun import -> LibEnv.find import lib_env) imports in
  Diag.fold disjoint_union init_env env_list

let chase_imports senv imports =
  let module S = Resolve_import.Set in
  let pending = ref S.empty in
  let lib_env = ref LibEnv.empty in
  let rec go file =
    if S.mem file !pending then
      error no_region "import" (sprintf "file %s must not depend on itself" file)
    else if LibEnv.mem file !lib_env then
      Diag.return ()
    else begin
        pending := S.add file !pending;
        let open Diag.Syntax in
        let* prog, base = parse_file file in
        let imports = Resolve_import.resolve prog base in
        let* () = go_set imports in
        let* base_env = merge_env imports senv !lib_env in
        let* scope, _ = check_prog base_env prog in
        lib_env := LibEnv.add file scope !lib_env;
        pending := S.remove file !pending;
        Diag.return ()
      end
  and go_set todo = Diag.traverse_ go todo
  in Diag.map (fun () -> !lib_env) (go_set imports)

let load_prog parse senv =
  let open Diag.Syntax in
  let* prog, base = parse in
  let imports = Resolve_import.resolve prog base in
  let* lib_env = chase_imports senv imports in
  let* base_env = merge_env imports senv lib_env in
  let* scope, actor = check_prog base_env prog in
  Diag.return (prog, scope, actor)

(* Only type checking *)

let initial_stat_env = Typing.empty_scope

let check_string source : load_result = load_prog (parse_string source) initial_stat_env
let check_file file : load_result = load_prog (parse_file file) initial_stat_env
let check_prog prog : Typing.scope Diag.result =
  let open Diag.Syntax in
  let* scope, _ = check_prog initial_stat_env prog in
  Diag.return scope

(* JS Compilation *)

type compile_result = Buffer.t Diag.result

let compile_js_file file : compile_result =
  let open Diag.Syntax in
  let* prog, senv, _ = check_file file in
  phase "JS Compiling" file;
  Diag.return (Compile_js.compile senv prog)

let compile_js_string source : compile_result =
  let open Diag.Syntax in
  let* prog, senv, _ = check_string source in
  phase "JS Compiling" "source3";
  Diag.return (Compile_js.compile senv prog)

(* Test file *)

type parse_test_file_result = Syntax.tests Diag.result

let parse_test_file filename : parse_test_file_result =
  let ic = open_in filename in
  let lexer = Lexing.from_channel ic in
  let parser = Parser.parse_tests in
  let name = Filename.basename filename in
  let result = parse_with lexer parser name in
  close_in ic;
  match result with
  | Ok prog -> Diag.return prog
  | Error e -> Error e

(* Values *)

let parse_values s =
  let lexer = Lexing.from_string s in
  lexer.Lexing.lex_curr_p <-
      {lexer.Lexing.lex_curr_p with Lexing.pos_fname = "(string)"};
  try
    Diag.return (Parser.parse_args Lexer.token lexer)
  with
    | ParseError (at, msg) ->
      error at "syntax" msg
    | Parser.Error ->
      error (Lexer.region lexer) "syntax" "unexpected token"
