open Mo_def
open Mo_frontend
open Mo_types
open Mo_config

let generic_parse_with mode lexer parser name : _ Diag.result =
  let open Diag.Syntax in
  lexer.Lexing.lex_curr_p <-
    {lexer.Lexing.lex_curr_p with Lexing.pos_fname = name};
  let tokenizer, triv_table = Lexer.tokenizer mode lexer in
  let* mk_syntax =
    try
      Parser_lib.triv_table := triv_table;
      Parsing.parse ~recovery:false mode (!Flags.error_detail) (parser lexer.Lexing.lex_curr_p) tokenizer lexer
    with Lexer.Error (at, msg) -> Diag.error at "M0002" "syntax" msg
  in
  Diag.return (mk_syntax name)

let parse_with mode lexer parser name : Syntax.prog Diag.result =
  generic_parse_with mode lexer parser name

let builtin_error phase what (msgs : Diag.messages) =
  Printf.eprintf "%s %s failed\n" phase what;
  Diag.print_messages msgs;
  exit 1

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
      Scope.adjoin senv0 sscope

let initial_stat_env0 =
  check_builtin "prelude" Prelude.prelude Typing.initial_scope

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
