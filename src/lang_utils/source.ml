type pos = Wasm.Source.pos = { file : string; line : int; column : int }
type region = Wasm.Source.region = { left : pos; right : pos }

open Wasm.Source

type ('a, 'b) annotated_phrase = {at : region; it : 'a; mutable note: 'b}
type 'a phrase = ('a, unit) annotated_phrase

module Pos_ord = struct
  type t = pos

  let compare l r =
    match compare l.file r.file with
    | 0 ->
      (match compare l.line r.line with
      | 0 -> compare l.column r.column
      | ord -> ord)
    | ord -> ord
end

module Region_ord = struct
  type t = region

  let compare l r =
    match Pos_ord.compare l.left r.left with
    | 0 -> Pos_ord.compare l.right r.right
    | ord -> ord
end

module Region_set = Set.Make (Region_ord)
module Region_map = Map.Make (Region_ord)

let annotate note it at = {it; at; note}
let (@@) it at = annotate () it at

(* Positions and regions *)

let no_pos = {file = ""; line = 0; column = 0}
let no_region = {left = no_pos; right = no_pos}

let is_no_pos p = p.file = "" && p.line = 0 && p.column = 0
let is_no_region r = is_no_pos r.left && is_no_pos r.right

let span r1 r2 = {left = r1.left; right = r2.right}
let between r1 r2 = {left = r1.right; right = r2.left}

let string_of_pos pos =
  if pos.line = -1 then
    Printf.sprintf "0x%x" pos.column
  else
    string_of_int pos.line ^ "." ^ string_of_int (pos.column + 1)

let string_of_region r =
  if r.left.file = "" then "(unknown location)" else
  r.left.file ^ ":" ^ string_of_pos r.left ^
  (if r.right = r.left then "" else "-" ^ string_of_pos r.right)

(* generic parse error *)

exception ParseError of region * string
