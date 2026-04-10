(* This program runs the Candid test suite at
   https://github.com/dfinity/candid/tree/master/test

*)
open Idllib
open Source
open Syntax
open Mo_idl
open Printf
open Mo_types
open Mo_values

let name = "candid-tests"
let version = "0.1"
let banner = "Candid test suite runner " ^ version ^ ""
let usage = "Usage: " ^ name ^ " [ -i path/to/candid/test ]"
let _WASMTIME_OPTIONS_ = "-C cache=n -W nan-canonicalization,memory64,multi-memory,bulk-memory"

(* Argument handling *)

let test_dir = ref ""
let pattern = ref ""
let expect_fail_pats = ref []
let diagnose = ref false

let print_banner () =
  printf "%s\n" banner;
  exit 0

let usage_err s =
  eprintf "%s: %s\n" name s;
  eprintf "%s\n" usage;
  exit 1

let argspec = Arg.align
[
  "-i", Arg.Set_string test_dir, " candid test directory";
  "-p", Arg.Set_string pattern, " test selector (substring/regex)";
  "--diag", Arg.Unit (fun () -> diagnose := true), " dump generated Motoko code for the tests to stderr";
  "--expect-fail", Arg.String (fun s -> expect_fail_pats := s :: !expect_fail_pats), " tests expected to fail";
  "--version", Arg.Unit print_banner, " show version";
]

(* IO *)

module Pretty = Type.MakePretty(Type.ElideStamps)

let write_file f s =
  let oc_ = open_out f in
  output_string oc_ s;
  close_out oc_

let print_type = function
  | [t] -> Pretty.string_of_typ t |> Idl_to_mo_value.parens (* add parens to make this unary *)
  | ts -> Pretty.string_of_typ (Type.Tup ts)

type expected_behaviour = ShouldPass | ShouldTrap

exception TextualParseError of (string * Diag.messages)

(* Turning a test case into a motoko program *)
let mo_of_test tenv test : (string * expected_behaviour, string) result =
  let deser ts = function
    | BinaryInput x ->
      "(prim \"deserialize\" : Blob -> " ^ print_type ts ^ ") " ^
      "\"" ^ Value.Blob.escape x ^ "\""
    | TextualInput x ->
      match Pipeline.parse_values x with
      | Error msgs -> raise (TextualParseError (x, msgs))
      | Ok (vals, _) -> Idl_to_mo_value.(args vals ts ^ " : " ^ print_type ts |> parens)
  in
  let equal e1 e2     = "assert " ^ e1 ^ " == " ^ e2 ^ "\n" in
  let not_equal e1 e2 = "assert " ^ e1 ^ " != " ^ e2 ^ "\n" in
  let ignore ts e =
    let open Type in
    if sub (seq ts) unit then e (* avoid warning about redundant ignore *)
    else "ignore " ^ e ^ "\n" in

  try
    let defs =
      "import _Prim \"mo:⛔\";\n" ^
      String.concat "" (List.map (fun (n,candid_typ) ->
        let mo_typ = Idl_to_mo.check_typ tenv candid_typ in
        "type " ^ n ^ " = " ^ Pretty.string_of_typ mo_typ ^ ";\n"
      ) (Typing.Env.bindings tenv)) ^ "\n" in

    let ts = Idl_to_mo.check_typs tenv (test.it.ttyp) in
    match test.it.assertion with
    | ParsesAs (_, TextualInput _)
    | ParsesEqual (_, TextualInput _, TextualInput _)
    -> Error "all-textual test case" (* not interesting to us *)
    | ParsesAs (true, i)
    -> Ok (defs ^ ignore ts (deser ts i), ShouldPass)
    | ParsesAs (false, i)
    -> Ok (defs ^ ignore ts (deser ts i), ShouldTrap)
    | ParsesEqual (true, i1, i2)
    -> Ok (defs ^ equal (deser ts i1) (deser ts i2), ShouldPass)
    | ParsesEqual (false, i1, i2)
    -> Ok (defs ^ not_equal (deser ts i1) (deser ts i2), ShouldPass)
  with
    | Exception.UnsupportedCandidFeature message ->
      Error (Diag.string_of_message message)
    | TextualParseError (x, msgs) ->
      Error (Printf.sprintf "Could not parse %S:\n%s" x
          (String.concat ", " (List.map Diag.string_of_message msgs))
      )

type result = Ok | Fail | Timeout

let run_cmd cmd : (result * string * string) =
  let  (stdout_c, stdin_c, stderr_c) =
    Unix.open_process_full cmd (Unix.environment ()) in
  (* shoddy reading of all pipes *)
  let s = Bytes.create 10000 in
  let n = input stdout_c s 0 10000 in
  let stdout = Bytes.sub_string s 0 n in
  let n = input stderr_c s 0 10000 in
  let stderr = Bytes.sub_string s 0 n in
  match Unix.close_process_full (stdout_c, stdin_c, stderr_c) with
  | Unix.WEXITED 0 -> (Ok, stdout, stderr)
  | Unix.WEXITED 124 -> (Timeout, stdout, stderr) (* see man timeout *)
  | _ -> (Fail, stdout, stderr)

type counts = {
  total : int ref;
  ok : int ref;
  fail : int ref;
  unexpected_ok : int ref;
  expected_fail : int ref;
  skip : int ref;
  ignored : int ref;
  }

let new_counts () : counts = {
  total = ref 0;
  ok = ref 0;
  fail = ref 0;
  unexpected_ok = ref 0;
  expected_fail = ref 0;
  skip = ref 0;
  ignored = ref 0;
  }

let bump r = r := !r + 1

type outcome =
 | WantedPass
 | WantedTrap
 | UnwantedPass
 | UnwantedTrap of (string * string) (* stdout, stderr *)
 | Timeout
 | Ignored of string
 | CantCompile of (string * string * string) (* stdout, stderr, src *)

let red s = "\027[31m" ^ s ^ "\027[0m"
let green s = "\027[32m" ^ s ^ "\027[0m"
let green2 s = "\027[32;1m" ^ s ^ "\027[0m"
let grey s = "\027[37m" ^ s ^ "\027[0m"

let report_outcome counts expected_fail outcome =
  match expected_fail, outcome with
  | false, WantedPass ->
    bump counts.ok;
    Printf.printf " %s\n" (green "ok (pass)");
  | true, WantedPass ->
    bump counts.unexpected_ok;
    Printf.printf " ok (pass) %s\n" (red "(unexpected!)");
  | false, WantedTrap ->
    bump counts.ok;
    Printf.printf " %s\n" (green "ok (trap)");
  | true, WantedTrap ->
    bump counts.unexpected_ok;
    Printf.printf " ok (trap) %s\n" (red "(despite --expect-fail)")
  | false, UnwantedPass ->
    bump counts.fail;
    Printf.printf " %s\n" (red "not ok (unwanted pass)!")
  | true, UnwantedPass ->
    bump counts.expected_fail;
    Printf.printf " not ok (unwanted pass) %s\n" (green2 "(expected)")
  | false, UnwantedTrap (stdout, stderr) ->
    bump counts.fail;
    Printf.printf " %s\n%s%s\n" (red "not ok (unwanted trap)!") stdout stderr
  | true, UnwantedTrap (stdout, stderr) ->
    bump counts.expected_fail;
    Printf.printf " not ok (unwanted trap) %s\n" (green2 "(expected)")
  | false, Timeout ->
    bump counts.fail;
    Printf.printf " %s\n" (red "not ok (timeout)")
  | true, Timeout  ->
    bump counts.expected_fail;
    Printf.printf " not ok (timeout) %s\n" (green2 "(expected)")
  | _, Ignored why ->
    bump counts.ignored;
    Printf.printf " %s\n" (grey (Printf.sprintf "ignored (%s)" why))
  | _, CantCompile (stdout, stderr, src) ->
    bump counts.fail;
    Printf.printf " %s\n%s%s\n%s" (red "not ok (cannot compile)") stdout stderr src

(* Main *)
let () =
  Arg.parse argspec (fun _ -> usage_err "no arguments expected") usage;
  if !test_dir = "" then
  begin
    match Sys.getenv_opt "CANDID_TESTS" with
    | Some path -> test_dir := path
    | None -> usage_err "no candid test directory specified via -i or $CANDID_TESTS";
  end;


  let filter =
    if !pattern = ""
    then fun name -> true
    else
      let r = Str.regexp !pattern in
      fun name ->
        try ignore(Str.search_forward r name 0); true
        with Not_found -> false
  in

  let expected_fail =
    let rs = List.map Str.regexp !expect_fail_pats in
    fun name ->
      List.exists (fun r ->
        try ignore(Str.search_forward r name 0); true
        with Not_found -> false
      ) rs
  in

  let counts = new_counts ()in

  let files = Sys.readdir !test_dir in
  Array.sort compare files;
  Array.iter (fun base ->
    match Lib.String.chop_suffix ".test.did" base with
    | Some name ->
      Printf.printf "Parsing %s ...\n%!" base;
      let tests = Diag.run (Pipeline.parse_test_file (Filename.concat !test_dir base)) in
      let tenv = Diag.run (Typing.check_tdecs Typing.empty_scope tests.it.tdecs) in

      List.iter (fun test ->
        let testname =
          match test.it.desc with
          | None -> Printf.sprintf "%s:%d" name test.at.left.line
          | Some n -> Printf.sprintf "%s:%d %s" name test.at.left.line n in

        bump counts.total;
        if filter testname then begin
          Printf.printf "%s ...%!" testname;
          (* generate test program *)
          let outcome =
          match mo_of_test tenv test with
          | Error why -> Ignored why
          | Ok (src, must_not_trap) ->
            if !diagnose then Printf.eprintf "## %s\n\n``` Motoko\n%s```\n" testname src;
            Unix.putenv "MOC_UNLOCK_PRIM" "yesplease";
            write_file "tmp.mo" src;
            match run_cmd "moc -A M0215 -Werror -wasi-system-api tmp.mo -o tmp.wasm" with
            | (Fail | Timeout), stdout, stderr -> CantCompile (stdout, stderr, src)
            | Ok, _, _ ->
              match must_not_trap, run_cmd ("timeout 10s wasmtime "^ _WASMTIME_OPTIONS_ ^" tmp.wasm") with
              | ShouldPass, (Ok, _, _) -> WantedPass
              | ShouldTrap, (Fail, _, _) -> WantedTrap
              | ShouldPass, (Fail, stdout, stderr) -> UnwantedTrap (stdout, stderr)
              | ShouldTrap, (Ok, _, _) -> UnwantedPass
              | _, (Timeout, _, _) -> Timeout
          in
          report_outcome counts (expected_fail testname) outcome
        end else
          bump counts.skip
      ) tests.it.tests;
    | None ->
      match Lib.String.chop_suffix ".did" base with
      | Some _ -> Printf.printf "Ignoring file %s ...\n" base;
      | None -> ()
  ) files;


  Printf.printf "%d tests:" !(counts.total);
  let res s r c =
    if !r > 0 then
    Printf.printf "  %s" (c (Printf.sprintf "%d %s" !r s)) in
  res "skipped" counts.skip grey;
  res "ok" counts.ok green;
  res "failed" counts.fail red;
  res "unexpected ok" counts.unexpected_ok red;
  res "expected fail" counts.expected_fail green2;
  res "ignored" counts.ignored grey;
  Printf.printf "\n";
  if !(counts.fail) + !(counts.unexpected_ok) > 0
  then exit 1
  else exit 0
