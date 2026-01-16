open Idllib.Syntax
open Idllib.Exception
open Source
module T = Mo_types.Type

module Pretty = T.MakePretty(T.ElideStamps)
(*
This module can translate Candid values (as parsed from the textual
representation) into Motoko values. This is a pragmatic translation which is
only used in the Candid test suite runner, to turn assertions about the
deserialized value into equalities between Motoko constants. It relies on the
fact that Candid and Motoko have very similar subtyping and overloading rules.

Unfortunately, the overloading rules are different enough that we need
to consider the expected Motoko type `t` as well.
*)

let parens s = "(" ^ s ^ ")"
let parens_comma ss = parens (String.concat ", " ss)
let brackets_comma ss = "[" ^ String.concat ", " ss ^ "]"

let is_tuple_rec fs =
  if fs = [] then false else
  let rec go next = function
    | [] -> true
    | f :: fs -> match (fst f.it).it with
      | Unnamed n when n = next -> go Lib.Uint32.(add n one) fs
      | Id n when n = next -> go Lib.Uint32.(add n one) fs
      | _ -> false
  in go Lib.Uint32.zero fs

(* We are lazy and encode text values like blobs. Output not pretty, but works. *)
let text_lit s = "\"" ^ Mo_values.Value.Blob.escape s ^ "\""

let find_typ ?(infer=fun _ -> None) tfs f =
  try T.lookup_val_field (Idl_to_mo.check_label (fst (f.it))) tfs
  with Invalid_argument _ ->
    match infer (snd (f.it)).it with
    | Some t -> t
    | _ ->
      raise (UnsupportedCandidFeature
               (Diag.error_message f.at "M0164" "import" "unknown record or variant label in textual representation"))

let infer_typ = function
  | NullV -> Some (T.(Prim Null))
  | NumV n when int_of_string n < 0 -> Some (T.(Prim Int))
  | NumV _ -> Some (T.(Prim Nat))
  | _ -> None


(* Also compare with Mo_values.Show.show_val, which we cannot use here, because
   we don’t have the full type (although we could have that), and we have
   overloaded numerals here.
*)
let rec value v t =
  match v.it, T.normalize t with
  | BoolV true, _ -> "true"
  | BoolV false, _ -> "false"
  | NullV, _ -> "null"
  | OptV v, T.Opt t1 -> parens ("?" ^ value v t1)
  | VecV vs, T.Array t1 -> brackets_comma (List.map (fun v -> value v t ) vs)
  | BlobV b, T.Prim T.Blob -> text_lit b
  | BlobV b, T.Array _ ->
    brackets_comma (List.of_seq (Seq.map (fun c -> Printf.sprintf "%d" (Char.code c)) (String.to_seq b)))
  | TextV s, _ -> text_lit s
  | RecordV fs, T.(Obj (Object, tfs, [])) ->
    "{" ^ String.concat "; " (List.map (fun f ->
      Idl_to_mo.check_label (fst f.it) ^ " = " ^ value (snd f.it) (find_typ ~infer:infer_typ tfs f)
    ) fs) ^ "}"
  | RecordV fs, T.Tup ts ->
    (* this will only line up if the record is written in tuple short hand order *)
    parens_comma (List.map2 (fun f t -> value (snd f.it) t) fs ts)
  | VariantV f, T.Variant tfs ->
    let t1 = find_typ tfs f in
    if T.normalize t1 = T.unit
    then parens ("#" ^ Idl_to_mo.check_label (fst f.it))
    else parens ("#" ^ Idl_to_mo.check_label (fst f.it) ^ parens (value (snd f.it) t1))
  | NumV n, _ -> n
  | ServiceV s, _ ->
    parens ("actor " ^ text_lit s)
  | FuncV (s, m), _ ->
    Printf.sprintf "(actor %s : actor { %s : %s }).%s"
      (text_lit s)
      (Idllib.Escape.escape_method Source.no_region m)
      (Pretty.string_of_typ t)
      (Idllib.Escape.escape_method Source.no_region m)
  | PrincipalV s, _ ->
    "_Prim.principalOfActor" ^ parens ("actor " ^ text_lit s ^ " : actor {}")
  | _ -> raise (UnsupportedCandidFeature
    (Diag.error_message v.at "M0165" "import" "odd expected type"))

let rec args vs = function
  | ts when List.(compare_lengths vs.it ts < 0 && for_all is_defaultable (Lib.List.drop (length vs.it) ts)) ->
    let vs' = vs.it @ Lib.List.replicate { vs with it = NullV } List.(length ts - length vs.it) in
    args {vs with it = vs'} ts
  | ts -> parens_comma (List.map2 value (List.map2 enrich ts vs.it) ts)
and is_defaultable t =
  T.(match normalize t with
     | Prim Null | Opt _ | Any -> true
     | _ -> false)
and enrich t v = match t, v.it with
  | T.(Obj (Object, tfs, _)), RecordV vfs ->
    let diff tfs vls = List.filter (fun T.{lab; typ; _} -> is_defaultable typ && not (List.mem lab vls)) tfs in
    let defaultable = diff tfs (List.map (fun {it; _} -> Idl_to_mo.check_label (fst it)) vfs) in
    let defaulted = List.map (fun T.{lab; _} -> { v with it = { v with it = Id (Idllib.Escape.unescape_hash lab) }, { v with it = NullV } }) defaultable in
    { v with it = RecordV (vfs @ defaulted) }
  | _ -> v
