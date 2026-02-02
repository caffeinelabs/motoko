(** Maintenance note:
    Update of the expected values could be done via [dune runtest --auto-promote].
    e.g. ~/motoko/src $ dune runtest mo_frontend --auto-promote
*)
module Parser = Mo_frontend.Parser
module Lexer = Mo_frontend.Lexer

let parse_from_lexbuf lexbuf : Mo_def.Syntax.prog Diag.result =
  let open Mo_frontend in
  let open Diag.Syntax in
  let name = "test.mo" in
  let lexer_mode = Lexer.mode in
  let lexer, triv_table = Lexer.tokenizer lexer_mode lexbuf in
  let () = Parser_lib.triv_table := triv_table in
  let start =  Parser.Incremental.parse_prog lexbuf.Lexing.lex_start_p in
  let error_details = 4 in
  let* mk_syntax = Mo_frontend.Parsing.parse ~recovery:true lexer_mode error_details start lexer lexbuf
  in Diag.return @@ mk_syntax name

let parse_from_string s : Mo_def.Syntax.prog Diag.result =
  let lexbuf = Lexing.from_string s in
  parse_from_lexbuf lexbuf

let show  (r: Mo_def.Syntax.prog Diag.result) : String.t =
  let show_msgs msgs =
    String.concat "\n" (List.map Diag.string_of_message msgs) in
  match r with
  | Ok (prog, msgs) ->
    "Ok: " ^ Wasm.Sexpr.to_string 80 (Mo_def.Arrange.prog prog)
    ^ "\n with errors:\n" ^ show_msgs msgs
  | Error msgs -> "Errors:\n" ^ show_msgs msgs

let show_with_types  (r: Mo_def.Syntax.prog Diag.result) : String.t =
  let show_msgs msgs =
    String.concat "\n" (List.map Diag.string_of_message msgs) in
  match r with
  | Ok (prog, msgs) ->
    let module Arrange = Mo_def.Arrange.Make(
      struct
        include Mo_def.Arrange.Default
        let include_types = true
      end
    ) in
    "Ok: " ^ Wasm.Sexpr.to_string 80 (Arrange.prog prog)
    ^ "\n with errors:\n" ^ show_msgs msgs
  | Error msgs -> "Errors:\n" ^ show_msgs msgs


let _parse_test input (expected : string) =
  let actual = parse_from_string input in
  if String.equal (show actual) expected then
    true
  else
    (Printf.printf "\nExpected:\n  %s\nbut got:\n  %s\n" (expected) (show actual); false)

let parse_and_typecheck_with_recovery s =
  match parse_from_string s with
  | Ok (prog, _) ->
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    (match Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog with
    | Ok (_, msgs) -> Ok (prog, msgs)
    | Error msgs -> Error msgs)
  | Error msgs -> Error msgs

let print_typed_ast s =
  Printf.printf "%s" @@ show_with_types (parse_and_typecheck_with_recovery s)

let%expect_test "test1" =
  let s = "actor {
  let x : Int = 1

  let y : Int = 2;

  let z : Int = 3

  let t : Int = 4
}" in
  Printf.printf "%s" @@ show (parse_from_string s);
  [%expect {|
    Ok: (Prog
      (ExpD
        (AwaitE
          (AsyncE
            _
            ($@anon-async-1.1 (PrimT Any))
            (ObjBlockE
              _
              Actor
              _
              (DecField
                (LetD
                  (VarP (ID x))
                  (AnnotE (LitE (PreLit 1 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
              (DecField
                (LetD
                  (VarP (ID y))
                  (AnnotE (LitE (PreLit 2 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
              (DecField
                (LetD
                  (VarP (ID z))
                  (AnnotE (LitE (PreLit 3 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
              (DecField
                (LetD
                  (VarP (ID t))
                  (AnnotE (LitE (PreLit 4 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
            )
          )
        )
      )
    )

     with errors:
    (unknown location): syntax error [M0001], unexpected token 'let', expected one of token or <phrase> sequence:
      }
      .<nat> (e.g. '.1')
      !
      <exp_nullary(ob)> (e.g. '42')
      <binop> <exp(ob)> (e.g. '+ 42')
      ; seplist(<dec_field>,<semicolon>) (e.g. '; public let x : Int = 0')
      |> <exp_bin(ob)> (e.g. '|> 42')
      or <exp_bin(ob)> (e.g. 'or 42')
      <unassign> <exp(ob)> (e.g. '-= 42')
      <relop> <exp_bin(ob)> (e.g. '== 42')
      else <exp_nest> (e.g. 'else 42')
      . <id>
      : <typ_nobin> (e.g. ': Int')
      := <exp(ob)> (e.g. ':= 42')
      <binop> <exp_bin(ob)> (e.g. '+ 42')
      <binassign> <exp(ob)> (e.g. '+= 42')
      and <exp_bin(ob)> (e.g. 'and 42')
      <unop> <exp_bin(ob)> (e.g. '- 42')
      <inst> <exp_nullary(ob)> (e.g. '<Int> 42')
      [ <exp(ob)> ] (e.g. '[ 42 ]')

    (unknown location): syntax error [M0001], unexpected token 'let', expected one of token or <phrase> sequence:
      }
      .<nat> (e.g. '.1')
      !
      <exp_nullary(ob)> (e.g. '42')
      <binop> <exp(ob)> (e.g. '+ 42')
      ; seplist(<dec_field>,<semicolon>) (e.g. '; public let x : Int = 0')
      |> <exp_bin(ob)> (e.g. '|> 42')
      or <exp_bin(ob)> (e.g. 'or 42')
      <unassign> <exp(ob)> (e.g. '-= 42')
      <relop> <exp_bin(ob)> (e.g. '== 42')
      else <exp_nest> (e.g. 'else 42')
      . <id>
      : <typ_nobin> (e.g. ': Int')
      := <exp(ob)> (e.g. ':= 42')
      <binop> <exp_bin(ob)> (e.g. '+ 42')
      <binassign> <exp(ob)> (e.g. '+= 42')
      and <exp_bin(ob)> (e.g. 'and 42')
      <unop> <exp_bin(ob)> (e.g. '- 42')
      <inst> <exp_nullary(ob)> (e.g. '<Int> 42')
      [ <exp(ob)> ] (e.g. '[ 42 ]') |}]

let%expect_test "test2" =
  let s = "actor {
  let x : Int = 1 +

  let y : Int = 2;
}" in
  Printf.printf "%s" @@ show (parse_from_string s);
  [%expect {|
    Ok: (Prog
      (ExpD
        (AwaitE
          (AsyncE
            _
            ($@anon-async-1.1 (PrimT Any))
            (ObjBlockE
              _
              Actor
              _
              (DecField
                (LetD
                  (VarP (ID x))
                  (AnnotE
                    (BinE ??? (LitE (PreLit 1 Nat)) AddOp (LoopE (BlockE)))
                    (PathT (IdH (ID Int)))
                  )
                )
                Private
                Flexible
              )
              (DecField
                (LetD
                  (VarP (ID y))
                  (AnnotE (LitE (PreLit 2 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
            )
          )
        )
      )
    )

     with errors:
    (unknown location): syntax error [M0001], unexpected token 'let', expected one of token or <phrase> sequence:
      <exp_bin(ob)> (e.g. '42') |}]

let%expect_test "test3" =
  let s = "actor {
  private func(a Int) {
    return
  }

  private func bar(y: Int) : Int { return 2; }

  let y : Int = 2;
}" in
  Printf.printf "%s" @@ show (parse_from_string s);
  [%expect {|
    Ok: (Prog
      (ExpD
        (AwaitE
          (AsyncE
            _
            ($@anon-async-1.1 (PrimT Any))
            (ObjBlockE
              _
              Actor
              _
              (DecField
                (ExpD
                  (FuncE
                    ???
                    Local
                    @anon-func-2.11
                    (TupP (VarP (ID a)) (VarP (ID Int)))
                    _

                    (BlockE (ExpD (RetE (TupE))))
                  )
                )
                Private
                (Flexible)
              )
              (DecField
                (LetD
                  (VarP (ID bar))
                  (FuncE
                    ???
                    Local
                    bar
                    (ParP (AnnotP (VarP (ID y)) (PathT (IdH (ID Int)))))
                    (PathT (IdH (ID Int)))

                    (BlockE (ExpD (RetE (LitE (PreLit 2 Nat)))))
                  )
                )
                Private
                Flexible
              )
              (DecField
                (LetD
                  (VarP (ID y))
                  (AnnotE (LitE (PreLit 2 Nat)) (PathT (IdH (ID Int))))
                )
                Private
                Flexible
              )
            )
          )
        )
      )
    )

     with errors:
    (unknown location): syntax error [M0001], unexpected token 'Int', expected one of token or <phrase> sequence:
      )
      or <pat_bin> (e.g. 'or x')
      , seplist(<pat_bin>,,) (e.g. ', x')
      : <typ> (e.g. ': Int')

    (unknown location): syntax error [M0001], unexpected token 'private', expected one of token or <phrase> sequence:
      }
      ; seplist(<dec_field>,<semicolon>) (e.g. '; public let x : Int = 0')

    (unknown location): syntax error [M0001], unexpected token 'let', expected one of token or <phrase> sequence:
      }
      ; seplist(<dec_field>,<semicolon>) (e.g. '; public let x : Int = 0') |}] 


let%expect_test "test4" =
  let s = "import { print } \"mo:base/Debug\";

actor Main {

    let x = 1

    public query test() : async Nat {
        123
    }
};" in
  Printf.printf "%s" @@ show (parse_from_string s);
  [%expect{|
    Ok: (Prog
      (LetD (ObjP (ValPF print (VarP (ID print)))) (ImportE mo:base/Debug))
      (LetD
        (VarP (ID Main))
        (AwaitE
          (AsyncE
            _
            ($@anon-async-3.1 (PrimT Any))
            (ObjBlockE
              _
              Actor
              Main
              (DecField (LetD (VarP (ID x)) (LitE (PreLit 1 Nat))) Private Flexible)
              (DecField
                (ExpD
                  (FuncE
                    ???
                    (Query (VarP (ID test)))
                    @anon-func-7.12
                    ($ (PrimT Any))
                    (TupP)
                    (AsyncT (PathT (IdH (ID $))) (PathT (IdH (ID Nat))))

                    (AsyncE
                      _
                      ($@anon-func-7.12 (PrimT Any))
                      (BlockE (ExpD (LitE (PreLit 123 Nat))))
                    )
                  )
                )
                Public
                (Flexible)
              )
            )
          )
        )
      )
    )

     with errors:
    (unknown location): syntax error [M0001], unexpected token 'public', expected one of token or <phrase> sequence:
      }
      .<nat> (e.g. '.1')
      !
      <exp_nullary(ob)> (e.g. '42')
      <binop> <exp(ob)> (e.g. '+ 42')
      ; seplist(<dec_field>,<semicolon>) (e.g. '; public let x : Int = 0')
      |> <exp_bin(ob)> (e.g. '|> 42')
      or <exp_bin(ob)> (e.g. 'or 42')
      <unassign> <exp(ob)> (e.g. '-= 42')
      <relop> <exp_bin(ob)> (e.g. '== 42')
      else <exp_nest> (e.g. 'else 42')
      . <id>
      : <typ_nobin> (e.g. ': Int')
      := <exp(ob)> (e.g. ':= 42')
      <binop> <exp_bin(ob)> (e.g. '+ 42')
      <binassign> <exp(ob)> (e.g. '+= 42')
      and <exp_bin(ob)> (e.g. 'and 42')
      <unop> <exp_bin(ob)> (e.g. '- 42')
      <inst> <exp_nullary(ob)> (e.g. '<Int> 42')
      [ <exp(ob)> ] (e.g. '[ 42 ]')

    (unknown location): syntax error [M0001], unexpected token '(', expected one of token or <phrase> sequence:
      func <func_pat> <annot_opt> <func_body> (e.g. 'func f(x : Int) : Int {}')
      class <func_pat> <annot_opt> <class_body> (e.g. 'class f(x : Int) : Int = {}')
      object class <func_pat> <annot_opt> <class_body> (e.g. 'object class f(x : Int) : Int = {}')
      module class <func_pat> <annot_opt> <class_body> (e.g. 'module class f(x : Int) : Int = {}')
      actor class <func_pat> <annot_opt> <class_body> (e.g. 'actor class f(x : Int) : Int = {}')
      persistent actor class <func_pat> <annot_opt> <class_body> (e.g. 'persistent actor class f(x : Int) : Int = {}') |}]

let%expect_test "test5" =
  let s = "module {

    let x =

    public query test() : async Nat {
        123
    }
};" in
  Printf.printf "%s" @@ show (parse_from_string s);
  [%expect{|
    Ok: (Prog
      (ExpD
        (ObjBlockE
          _
          Module
          _
          (DecField (LetD (VarP (ID x)) (LoopE (BlockE))) Private (Flexible))
          (DecField
            (ExpD
              (FuncE
                ???
                (Query (VarP (ID test)))
                @anon-func-5.12
                ($ (PrimT Any))
                (TupP)
                (AsyncT (PathT (IdH (ID $))) (PathT (IdH (ID Nat))))

                (AsyncE
                  _
                  ($@anon-func-5.12 (PrimT Any))
                  (BlockE (ExpD (LitE (PreLit 123 Nat))))
                )
              )
            )
            Public
            (Flexible)
          )
        )
      )
    )

     with errors:
    (unknown location): syntax error [M0001], unexpected token 'public', expected one of token or <phrase> sequence:
      <exp(ob)> (e.g. '42')
      <exp(ob)> else <exp_nest> (e.g. '42 else 42')

    (unknown location): syntax error [M0001], unexpected token '(', expected one of token or <phrase> sequence:
      func <func_pat> <annot_opt> <func_body> (e.g. 'func f(x : Int) : Int {}')
      class <func_pat> <annot_opt> <class_body> (e.g. 'class f(x : Int) : Int = {}')
      object class <func_pat> <annot_opt> <class_body> (e.g. 'object class f(x : Int) : Int = {}')
      module class <func_pat> <annot_opt> <class_body> (e.g. 'module class f(x : Int) : Int = {}')
      actor class <func_pat> <annot_opt> <class_body> (e.g. 'actor class f(x : Int) : Int = {}')
      persistent actor class <func_pat> <annot_opt> <class_body> (e.g. 'persistent actor class f(x : Int) : Int = {}') |}]

let%expect_test "test type recovery 1" =
  let s = "func test_func () {
  let x = Counter(0);
  x.
  let a = 1;
}

class Counter(n: Nat) {
  var counter : Nat = n;
  func inc() {counter +=1;};
  func get() : Nat {counter};
}" in
  match (parse_from_string s) with
  | Ok (prog, _) -> begin
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    match (Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog) with
    | Ok (_, msgs) ->
      Printf.printf "%s" @@ show_with_types (Ok (prog, msgs));
      [%expect {|
        Ok: (Prog
          (LetD
            (: (VarP (ID test_func)) () -> ())
            (:
              (FuncE
                () -> ()
                Local
                test_func
                (: (TupP) ())
                _

                (:
                  (BlockE
                    (LetD
                      (: (VarP (ID x)) {})
                      (:
                        (CallE
                          _
                          (: (VarE (ID Counter)) (n : Nat) -> Counter)
                          (: (LitE (NatLit 0)) Nat)
                        )
                        {}
                      )
                    )
                    (ExpD
                      (: (DotE (: (VarE (ID x)) {}) (ID __error_recovery_var__)) ???)
                    )
                    (LetD (: (VarP (ID a)) Nat) (: (LitE (NatLit 1)) Nat))
                  )
                  ()
                )
              )
              () -> ()
            )
          )
          (ClassD
            _
            Local
            (ID Counter)
            (:
              (ParP
                (: (AnnotP (: (VarP (ID n)) Nat) (: (PathT (IdH (ID Nat))) Nat)) Nat)
              )
              Nat
            )
            _
            Object
            (ID @anon-object-7.23)
            (DecField
              (VarD
                (ID counter)
                (: (AnnotE (: (VarE (ID n)) Nat) (: (PathT (IdH (ID Nat))) Nat)) Nat)
              )
              Private
              (Flexible)
            )
            (DecField
              (LetD
                (: (VarP (ID inc)) () -> ())
                (:
                  (FuncE
                    () -> ()
                    Local
                    inc
                    (: (TupP) ())
                    _

                    (:
                      (BlockE
                        (ExpD
                          (:
                            (AssignE
                              (: (VarE (ID counter)) var Nat)
                              (:
                                (BinE
                                  Nat
                                  (: (VarE (ID counter)) Nat)
                                  AddOp
                                  (: (LitE (NatLit 1)) Nat)
                                )
                                Nat
                              )
                            )
                            ()
                          )
                        )
                      )
                      ()
                    )
                  )
                  () -> ()
                )
              )
              Private
              (Flexible)
            )
            (DecField
              (LetD
                (: (VarP (ID get)) () -> Nat)
                (:
                  (FuncE
                    () -> Nat
                    Local
                    get
                    (: (TupP) ())
                    (: (PathT (IdH (ID Nat))) Nat)

                    (: (BlockE (ExpD (: (VarE (ID counter)) Nat))) Nat)
                  )
                  () -> Nat
                )
              )
              Private
              (Flexible)
            )
          )
        )

         with errors:
        (unknown location): type error [M0072], field __error_recovery_var__ does not exist in type:
          {}
      |}]
    | Error msgs -> Printf.printf "%s" @@ show (Error msgs)
    end
  | Error _ as r -> Printf.printf "%s" @@ show r;
  [%expect.unreachable]

let%expect_test "test type recovery 2" =
  let s = "module M {};
let _x = M.
" in
  match (parse_from_string s) with
  | Ok (prog, _) -> begin
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    match (Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog) with
    | Ok (_, msgs) ->
      Printf.printf "%s" @@ show_with_types (Ok (prog, msgs));
      [%expect {|
        Ok: (Prog
          (LetD (: (VarP (ID M)) module {}) (: (ObjBlockE _ Module _) ???))
          (LetD
            (: (VarP (ID _x)) ???)
            (: (DotE (: (VarE (ID M)) ???) (ID __error_recovery_var__)) ???)
          )
        )

         with errors:
        (unknown location): type error [M0072], field __error_recovery_var__ does not exist in type:
          module {}
      |}]
    | Error msgs -> Printf.printf "%s" @@ show (Error msgs)
    end
  | Error _ as r -> Printf.printf "%s" @@ show r;
  [%expect.unreachable]

let%expect_test "test type recovery 3" =
  let s = "let _x = (1 +
" in
  match (parse_from_string s) with
  | Ok (prog, _) -> begin
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    match (Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog) with
    | Ok (_, msgs) ->
      Printf.printf "%s" @@ show_with_types (Ok (prog, msgs));
      [%expect {|
        Ok: (Prog
          (LetD
            (: (VarP (ID _x)) Nat)
            (:
              (BinE
                Nat
                (: (LitE (NatLit 1)) Nat)
                AddOp
                (: (LoopE (: (BlockE) ())) None)
              )
              Nat
            )
          )
        )

         with errors:
      |}]
    | Error msgs -> Printf.printf "%s" @@ show (Error msgs)
    end
  | Error _ as r -> Printf.printf "%s" @@ show r;
  [%expect.unreachable]

let%expect_test "test type recovery 4" =
  let s = "f(x
" in
  match (parse_from_string s) with
  | Ok (prog, _) -> begin
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    match (Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog) with
    | Ok (_, msgs) ->
      Printf.printf "%s" @@ show_with_types (Ok (prog, msgs));
      [%expect {|
        Ok: (Prog (ExpD (: (CallE _ (: (VarE (ID f)) ???) (: (VarE (ID x)) ???)) ???)))

         with errors:
        (unknown location): type error [M0057], unbound variable f
      |}]
    | Error msgs -> Printf.printf "%s" @@ show (Error msgs)
    end
  | Error _ as r -> Printf.printf "%s" @@ show r;
  [%expect.unreachable]

let%expect_test "test type recovery 5" =
  let s = "import A \"a\";
  A.f(x
" in
  match (parse_from_string s) with
  | Ok (prog, _) -> begin
    let open Mo_frontend in
    let async_cap = Pipeline.async_cap_of_prog prog in
    match (Typing.infer_prog ~enable_type_recovery:true Pipeline.initial_stat_env None async_cap prog) with
    | Ok (_, msgs) ->
      Printf.printf "%s" @@ show_with_types (Ok (prog, msgs));
      [%expect {|
        Ok: (Prog
          (LetD (: (VarP (ID A)) ???) (: (ImportE a) ???))
          (ExpD
            (:
              (CallE
                _
                (: (DotE (: (VarE (ID A)) ???) (ID f)) ???)
                (: (VarE (ID x)) ???)
              )
              ???
            )
          )
        )

         with errors:
        (unknown location): type error [M0020], unresolved import a
      |}]
    | Error msgs -> Printf.printf "%s" @@ show (Error msgs)
    end
  | Error _ as r -> Printf.printf "%s" @@ show r;
  [%expect.unreachable]

let%expect_test "context dot callee function type should not be ???" =
  print_typed_ast "func foo(self : [Nat]) : Text { \"foo\" };
let ar = [1];
ar.foo();";
  [%expect {|
    Ok: (Prog
      (LetD
        (: (VarP (ID foo)) (self : [Nat]) -> Text)
        (:
          (FuncE
            (self : [Nat]) -> Text
            Local
            foo
            (:
              (ParP
                (:
                  (AnnotP
                    (: (VarP (ID self)) [Nat])
                    (: (ArrayT Const (: (PathT (IdH (ID Nat))) Nat)) [Nat])
                  )
                  [Nat]
                )
              )
              [Nat]
            )
            (: (PathT (IdH (ID Text))) Text)

            (: (BlockE (ExpD (: (LitE (TextLit foo)) Text))) Text)
          )
          (self : [Nat]) -> Text
        )
      )
      (LetD
        (: (VarP (ID ar)) [Nat])
        (: (ArrayE Const (: (LitE (NatLit 1)) Nat)) [Nat])
      )
      (ExpD
        (:
          (CallE
            _
            (:
              (DotE
                (: (VarE (ID ar)) [Nat])
                (: (VarE (ID foo)) (self : [Nat]) -> Text)
              )
              (self : [Nat]) -> Text
            )
            (: (TupE) ())
          )
          Text
        )
      )
    )

     with errors:
    (unknown location): warning [M0194], unused identifier self (delete or rename to wildcard `_` or `_self`)
  |}]
