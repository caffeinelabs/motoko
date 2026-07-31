(*
This module implements the field name mapping from Motoko to IDL
and back.

(It could live in mo_idl/, but mo_idl depends on mo_def/, codegen/ needs this
encoding, but codegen/ should not depend on mo_def/.)
*)

(* Utilities (cf. lexer.mll) *)

let is_underscore c = c = '_'
let is_letter c = ('a' <= c && c <= 'z') || ('A' <= c && c <= 'Z')
let is_digit c = '0' <= c && c <= '9'

let is_valid_as_id str = match Lib.String.explode str with
  | c::cs ->
    (is_underscore c || is_letter c) &&
    List.for_all (fun c -> is_underscore c || is_letter c || is_digit c) cs
  | _ -> false

let ends_with_underscore str = Lib.String.chop_suffix "_" str <> None

let is_candid_keyword = function
  | "import"
  | "service"
  | "func"
  | "type"
  | "opt"
  | "vec"
  | "record"
  | "variant"
  | "blob"
  | "principal"
  | "nat"
  | "nat8"
  | "nat16"
  | "nat32"
  | "nat64"
  | "int"
  | "int8"
  | "int16"
  | "int32"
  | "int64"
  | "float32"
  | "float64"
  | "bool"
  | "text"
  | "null"
  | "reserved"
  | "empty"
  | "oneway"
  | "query"
  | "composite_query"
  -> true
  | _
  -> false

let is_motoko_keyword = function
  | "actor"
  | "and"
  | "async"
  | "async*"
  | "assert"
  | "await"
  | "await*"
  | "break"
  | "case"
  | "catch"
  | "class"
  | "continue"
  | "composite"
  | "debug"
  | "debug_show"
  | "do"
  | "else"
  | "false"
  | "finally"
  | "flexible"
  | "for"
  | "from_candid"
  | "func"
  | "if"
  | "ignore"
  | "implicit"
  | "import"
  | "in"
  | "include"
  | "mixin"
  | "module"
  | "not"
  | "null"
  | "object"
  | "or"
  | "label"
  | "let"
  | "loop"
  | "persistent"
  | "private"
  | "public"
  | "query"
  | "return"
  | "shared"
  | "stable"
  | "switch"
  | "system"
  | "transient"
  | "try"
  | "throw"
  | "to_candid"
  | "true"
  | "type"
  | "var"
  | "weak"
  | "while"
  | "with"
  -> true
  | _
  -> false

(* Escaping (used for Candid → Motoko) *)

let escape_num h = Printf.sprintf "_%s_" (Lib.Uint32.to_string h)

(* Snake / lower identifiers → PascalCase Motoko type names.
   Only ids starting with a lowercase letter are transformed;
   inner capitalization is preserved, so acronyms ("HTTP_request", "getSNS") survive intact. *)
let pascal_case str =
  match Lib.String.explode str with
  | c :: _ when 'a' <= c && c <= 'z' ->
    let buf = Buffer.create (String.length str) in
    let capitalize = ref true in
    String.iter (fun c ->
      if is_underscore c then capitalize := true
      else begin
        Buffer.add_char buf (if !capitalize then Char.uppercase_ascii c else c);
        capitalize := false
      end) str;
    Buffer.contents buf
  | _ -> str

let escape str =
  if is_motoko_keyword str then str ^ "_" else
  if is_valid_as_id str
  then if ends_with_underscore str then str ^ "_" else str
  else escape_num (IdlHash.idl_hash str)

let escape_method at str =
  if is_motoko_keyword str then str ^ "_" else
  if is_valid_as_id str
  then if ends_with_underscore str then str ^ "_" else str
  else raise (Exception.UnsupportedCandidFeature
    (Diag.error_message at "M0160" "import"
      (Printf.sprintf "Candid method name '%s' is not a valid Motoko identifier" str)))

(* Unescaping (used for Motoko → Candid) *)

let is_escaped_num str =
  match Lib.String.chop_prefix "_" str with
  | Some str ->
    begin match Lib.String.chop_suffix "_" str with
    | Some str
      when String.length str > 0 &&
           List.for_all is_digit (Lib.String.explode str)
      -> Lib.Uint32.of_string_opt str
    | _ -> None
    end
  | _ -> None

(* This (or a type like this) could move into the IDL AST *)
type label = Nat of Lib.Uint32.t | Id of string

let unescape str =
  match is_escaped_num str with
  | Some h -> Nat h
  | _ -> match Lib.String.chop_suffix "_" str with
    | Some str' -> Id str'
    | _ -> Id str

let unescape_hash str = match unescape str with
  | Nat h -> h
  | Id s -> IdlHash.idl_hash s

let unescape_method str =
  match Lib.String.chop_suffix "_" str with
    | Some str' -> str'
    | _ -> str

let needs_candid_quote str =
  not (is_valid_as_id str) || is_candid_keyword str
