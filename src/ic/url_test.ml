open Url

let parse_test input expected =
  let actual = parse input in
  let show = function
    | Result.Ok parsed -> "Ok " ^ string_of_parsed parsed
    | Result.Error err -> "Err " ^ err in
  if actual = expected then
    true
  else
    (Printf.printf "\nExpected:\n  %s\nbut got:\n  %s\n" (show expected) (show actual); false)

let%test "it should parse a package import" =
  parse_test "mo:std/list" (Ok (Package ("std", "list")))
let%test "it should parse a package import (2)" =
  parse_test "mo:std/foo/bar" (Ok (Package ("std", "foo/bar")))
let%test "it should parse a package import (3)" =
  parse_test "mo:foo/bar" (Ok (Package ("foo", "bar")))
let%test "it should parse a package import (4)" =
  parse_test "mo:foo" (Ok (Package ("foo", "")))
let%test "it should reject package imports that try to escape" =
  parse_test "mo:foo/../bar" (Error "Package imports mustn't access parent directories: ../bar is invalid.")
let%test "it should reject package imports that try to escape (2)" =
  parse_test "mo:foo/bar/../../baz" (Error "Package imports mustn't access parent directories: bar/../../baz is invalid.")
let%test "it normalises filepaths for the escape check (3)" =
  parse_test "mo:foo/bar/../baz" (Ok (Package ("foo", "bar/../baz")))

let%test "it should parse a prim import" =
  parse_test "mo:⛔" (Ok (Prim))
let%test "it should fail to parse a malformed prim import" =
  (* TODO This should be erroring *)
  (* parse_test "mo:⛔/bar" (Error "") *)
  parse_test "mo:⛔/bar" (Ok (Package ("⛔", "bar")))

let%test "it should parse an ic import" =
  parse_test "ic:5h74t-uga73-7nadi" (Error "invalid principal. Did you mean \"bfozs-kwa73-7nadi\"?")

let%test "it should parse a canister alias import" =
  parse_test "canister:foo" (Ok (IcAlias "foo"))

let%test "it should parse a relative import" =
  parse_test "std/foo" (Ok (Relative "std/foo"))
let%test "it should parse a relative import (2)" =
  parse_test "foo" (Ok (Relative "foo"))
let%test "it should parse a relative import (3)" =
  parse_test "./foo" (Ok (Relative "foo"))
let%test "it should parse a raw file value import" =
  parse_test "blob:file:foo" (Ok (FileValue "foo"))
let%test "it should fail to parse an unknown URL scheme" =
  parse_test "something:else" (Error "Unrecognized URL")
let%test "it should parse an idl file import" =
  parse_test "idl:foo.did" (Ok (IdlFile "foo.did"))
let%test "it should parse an idl file import with path" =
  parse_test "idl:./api/ledger.did" (Ok (IdlFile "./api/ledger.did"))
