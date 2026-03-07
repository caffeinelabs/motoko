(* Parsing known URLs from mo:, ic:, canister: and file: URLs *)

(*
   For usage examples take a look at url_test.ml
*)

let checkbytes s : string =
  let buf = Buffer.create 4 in
  Buffer.add_int32_be buf (Lib.CRC.crc32 s); (* NB: big endian *)
  Buffer.contents buf

let rec group s =
  let open String in
  if length s <= 5 then s else
  sub s 0 5 ^ "-" ^ group (sub s 5 (length s - 5))

let encode_principal bytes : string =
  group (String.map Char.lowercase_ascii (Lib.Base32.encode (checkbytes bytes ^ bytes)))

(* Decode a principal according to https://sdk.dfinity.org/docs/interface-spec/index.html#textual-ids *)
let decode_principal principal : (string, string) result =
  let open Stdlib.String in

  if principal = "" then Error "principal cannot be empty" else
  let filtered =
    to_seq principal |>
      Seq.map Char.uppercase_ascii |>
      Seq.filter (fun c -> c >= '0' && c <= '9' || c >= 'A' && c <= 'Z') |>
      of_seq in
  match Lib.Base32.decode filtered with
  | Error e -> Error e
  | Ok bytes ->
    if length bytes < 4 then Error "principal too short" else
    let payload = sub bytes 4 (length bytes - 4) in
    let expected = encode_principal payload in
    if principal <> expected
    then Error (Printf.sprintf "invalid principal. Did you mean %S?" expected)
    else Ok payload

type parsed =
  | Package of (string * string)
  | Relative of string
  | Ic of string
  | IcAlias of string
  | FileValue of string
  | Prim

let string_of_parsed = function
  | Package (x, y) -> Printf.sprintf "Package (%s, %s)" x y
  | Relative x -> Printf.sprintf "Relative %s" x
  | Ic x -> Printf.sprintf "Ic %s" x
  | IcAlias x -> Printf.sprintf "IcAlias %s" x
  | FileValue x -> Printf.sprintf "FileValue %s" x
  | Prim -> "Prim"

let parse (f: string) : (parsed, string) result =
  match Lib.String.chop_prefix "mo:" f with
  | Some suffix ->
    begin match Stdlib.String.index_opt suffix '/' with
    | None ->
      if suffix = "prim" || suffix = "⛔"
      then Ok Prim
      else Ok (Package (suffix, ""))
    | Some i ->
      if suffix = "prim" || suffix = "⛔"
      then Error "The prim package has no modules"
      else
        let pkg = Stdlib.String.sub suffix 0 i in
        let path = Stdlib.String.sub suffix (i+1) (Stdlib.String.length suffix - (i+1)) in
        if Option.is_some (Lib.String.chop_prefix ".." (Lib.FilePath.normalise path))
        then Error (Printf.sprintf "Package imports mustn't access parent directories: %s is invalid." path)
        else Ok (Package (pkg, path))
    end
  | None ->
    match Lib.String.chop_prefix "ic:" f with
    | Some principal -> begin match decode_principal principal with
      | Ok bytes -> Ok (Ic bytes)
      | Error err -> Error err
      end
    | None ->
      match Lib.String.chop_prefix "canister:" f with
      | Some suffix -> Ok (IcAlias suffix)
      | None ->
        match Lib.String.chop_prefix "blob:file:" f with
        | Some suffix -> Ok (FileValue suffix)
        | None ->
          match Stdlib.String.index_opt f ':' with
          | Some _ -> Error "Unrecognized URL"
          | None -> Ok (Relative (Lib.FilePath.normalise f))


(* Basename of the IDL file searched (see DFX-Interface.md) *)
let idl_extension = ".did"

let idl_basename_of_blob bytes =
  encode_principal bytes ^ idl_extension

let idl_basename_of_envvar envvar =
  envvar ^ idl_extension
