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
    5.18-5.29 [M0237]:
      `Nat.compare, ` ~> `` (5.18-5.31) |}]

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
    7.2-7.13 [M0237]:
      `Nat.compare,
      ` ~> `` (7.2-8.2) |}]

let%expect_test "M0236: complex receiver" =
  print_edits "m0236_complex_receiver.mo";
  [%expect {|
    3.7-5.1 [M0236]:
      `Map` ~> `Map.empty<Nat, Text>()` (3.7-3.10)
      `Map.empty<Nat, Text>()` ~> `` (4.2-4.24) |}]

let%expect_test "M0223: redundant instantiation" =
  print_edits "m0223.mo";
  [%expect {|
    4.21-4.26 [M0223]:
      `<Nat>` ~> `` (4.21-4.26) |}]

let%expect_test "M0237: single-line" =
  print_edits "m0237_single_line.mo";
  [%expect {|
    5.13-5.24 [M0237]:
      `Nat.compare, ` ~> `` (5.13-5.26) |}]

let%expect_test "M0237: multi-line" =
  print_edits "m0237_multiline.mo";
  [%expect {|
    6.2-6.13 [M0237]:
      `Nat.compare,
      ` ~> `` (6.2-7.2) |}]

let%expect_test "M0237: complex implicits" =
  print_edits "m0237_complex.mo";
  [%expect {|
    61.7-61.34 [M0236]:
      `M` ~> `data` (61.7-61.8)
      `data, ` ~> `` (61.13-61.19)
    61.19-61.30 [M0237]:
      `Nat.compare, ` ~> `` (61.19-61.32)
    64.7-64.53 [M0236]:
      `M` ~> `data` (64.7-64.8)
      `data, ` ~> `` (64.13-64.19)
    64.19-64.30 [M0237]:
      `Nat.compare, ` ~> `` (64.19-64.32)
    64.32-64.44 [M0237]:
      `Text.compare, ` ~> `` (64.32-64.46)
    67.7-67.35 [M0236]:
      `M` ~> `data` (67.7-67.8)
      `data, ` ~> `` (67.14-67.20)
    67.23-67.34 [M0237]:
      `Nat.compare` ~> `` (67.23-67.34)
    69.7-69.33 [M0236]:
      `M` ~> `data` (69.7-69.8)
      `data, ` ~> `` (69.15-69.21)
    69.21-69.32 [M0237]:
      `Nat.compare` ~> `` (69.21-69.32)
    71.21-71.32 [M0237]:
      `Nat.compare` ~> `` (71.21-71.32)
    75.26-75.37 [M0237]:
      `Nat.compare, ` ~> `` (75.26-75.39)
    75.39-75.51 [M0237]:
      `Text.compare` ~> `` (75.39-75.51)
    78.7-78.56 [M0236]:
      `M` ~> `data` (78.7-78.8)
      `data, ` ~> `` (78.16-78.22)
    78.22-78.33 [M0237]:
      `Nat.compare, ` ~> `` (78.22-78.35)
    78.38-78.50 [M0237]:
      `Text.compare, ` ~> `` (78.38-78.52)
    81.7-87.1 [M0236]:
      `M` ~> `data` (81.7-81.8)
      `data,
      ` ~> `` (82.2-83.2)
    83.2-83.13 [M0237]:
      `Nat.compare,
      ` ~> `` (83.2-84.2)
    84.2-84.14 [M0237]:
      `Text.compare,
      ` ~> `` (84.2-85.2)
    89.7-95.1 [M0236]:
      `M` ~> `data` (89.7-89.8)
      `data,
       ` ~> `` (90.2-91.3)
    91.3-91.14 [M0237]:
      `Nat.compare, // expected: edit would remove this comment
         ` ~> `` (91.3-92.5)
    92.5-92.17 [M0237]:
      `Text.compare,
        ` ~> `` (92.5-93.4) |}]

let%expect_test "M0223 + M0236 + M0237: redundant type instantiation + dot + implicit" =
  print_edits "mix.mo";
  [%expect {|
    4.15-4.26 [M0223]:
      `<Nat, Text>` ~> `` (4.15-4.26)
    4.8-9.1 [M0236]:
      `Map` ~> `Map.empty<Nat, Text>()` (4.8-4.11)
      `Map.empty<Nat, Text>(),
      ` ~> `` (5.2-6.2)
    6.2-6.13 [M0237]:
      `Nat.compare,
      ` ~> `` (6.2-7.2)
    4.0-4.8 [M0239]: |}]
