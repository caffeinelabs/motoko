(** Tests for suggested edits in diagnostics.

    Maintenance note:
    Update expected values via [dune runtest --auto-promote].
    e.g. ~/motoko/src $ dune runtest mo_frontend --auto-promote
*)

module Flags = Mo_config.Flags

let test_dir = "test_edits/"

let motoko_core =
  try Sys.getenv "MOTOKO_CORE"
  with Not_found -> failwith "MOTOKO_CORE environment variable not set"

let typecheck_file filename =
  let saved_warnings = !Flags.warning_levels in
  let saved_packages = !Flags.package_urls in
  Flags.set_warning_level "M0223" Flags.Warn;
  Flags.set_warning_level "M0236" Flags.Warn;
  Flags.set_warning_level "M0237" Flags.Warn;
  Flags.package_urls := Flags.M.add "core" motoko_core !Flags.package_urls;
  let path = test_dir ^ filename in
  let result =
    Pipeline.load_progs Pipeline.parse_file [path] Pipeline.initial_stat_env
  in
  Flags.warning_levels := saved_warnings;
  Flags.package_urls := saved_packages;
  result

let short_region (r : Source.region) =
  Printf.sprintf "%d.%d-%d.%d" r.left.line r.left.column r.right.line r.right.column

let assert_file filename (r : Source.region) =
  let path = test_dir ^ filename in
  if r.left.file <> path then
    Printf.printf "UNEXPECTED FILE: expected %s, got %s\n" path r.left.file

let print_edits filename =
  let msgs = match typecheck_file filename with
    | Ok (_, msgs) -> msgs
    | Error msgs -> msgs
  in
  List.iter (fun (m : Diag.message) ->
    assert_file filename m.at;
    Printf.printf "%s [%s]:\n" (short_region m.at) m.code;
    List.iter (fun (e : Diag.text_edit) ->
      assert_file filename e.Source.at;
      let before = match Source.read_region e.Source.at with
        | Some s -> s
        | None -> "???"
      in
      Printf.printf "  `%s` ~> `%s` (%s)\n" before e.Source.it (short_region e.Source.at)
    ) m.edits
  ) msgs

let%expect_test "M0236: single-arg" =
  print_edits "m0236_single_arg.mo";
  [%expect {|
    4.7-4.18 [M0236]:
      `Map` ~> `m` (4.7-4.10)
      `m` ~> `` (4.16-4.17) |}]

let%expect_test "M0236: multi-arg with implicit" =
  print_edits "m0236_multi_arg_implicit.mo";
  [%expect {|
    5.7-5.33 [M0236]:
      `Map` ~> `m` (5.7-5.10)
      `m, ` ~> `` (5.15-5.18)
    5.18-5.29 [M0237]: |}]

let%expect_test "M0236: multi-arg without implicit" =
  print_edits "m0236_multi_arg.mo";
  [%expect {|
    4.7-4.20 [M0230]:
    4.7-4.20 [M0236]:
      `Map` ~> `m` (4.7-4.10)
      `m, ` ~> `` (4.15-4.18) |}]

let%expect_test "M0236: multi-arg multiline" =
  print_edits "m0236_multiline.mo";
  [%expect {|
    5.0-10.1 [M0236]:
      `Map` ~> `m` (5.0-5.3)
      `m,
      ` ~> `` (6.2-7.2)
    7.2-7.13 [M0237]: |}]

let%expect_test "M0236: complex receiver" =
  print_edits "m0236_complex_receiver.mo";
  [%expect {|
    3.7-5.1 [M0236]:
      `Map` ~> `Map.empty<Nat, Text>()` (3.7-3.10)
      `Map.empty<Nat, Text>()` ~> `` (4.2-4.24) |}]

let%expect_test "M0223: redundant instantiation" =
  print_edits "m0223.mo";
  [%expect {| 4.21-4.26 [M0223]: |}]

let%expect_test "M0237: single-line" =
  print_edits "m0237_single_line.mo";
  [%expect {| 5.13-5.24 [M0237]: |}]

let%expect_test "M0237: multi-line" =
  print_edits "m0237_multiline.mo";
  [%expect {| 6.2-6.13 [M0237]: |}]

let%expect_test "M0236 + M0237: dot + implicit" =
  print_edits "mix_m0236_m0237.mo";
  [%expect {|
    4.0-9.1 [M0236]:
      `Map` ~> `Map.empty<Nat, Text>()` (4.0-4.3)
      `Map.empty<Nat, Text>(),
      ` ~> `` (5.2-6.2)
    6.2-6.13 [M0237]: |}]
