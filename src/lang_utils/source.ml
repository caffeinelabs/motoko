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

(* read source code from region *)
let mark_line line start_marker end_marker =
  let len = String.length line in
  match (start_marker, end_marker) with
  | (Some start_col, Some end_col) ->
      String.sub line 0 start_col ^ "**" ^
      String.sub line start_col (end_col - start_col) ^ "**" ^
      String.sub line end_col (len - end_col)
  | (Some start_col, None) ->
      String.sub line 0 start_col ^ "**" ^ String.sub line start_col (len - start_col)
  | (None, Some end_col) ->
      String.sub line 0 end_col ^ "**" ^ String.sub line end_col (len - end_col)
  | (None, None) -> line

let extract_part line start_marker end_marker =
  let len = String.length line in
  let start_col = Option.value start_marker ~default:0 in
  let end_col = Option.value end_marker ~default:len in
  String.sub line start_col (end_col - start_col)

let read_region_with process_line r =
  try
    let in_channel = open_in r.left.file in
    let rec skip_lines n =
      if n > 0 then begin
        ignore (input_line in_channel);
        skip_lines (n - 1)
      end
    in
    let rec read_lines start_line end_line acc =
      if start_line > end_line then
        String.concat "\n" (List.rev acc)
      else
        let line = input_line in_channel in
        let marked_line =
          if start_line = r.left.line && start_line = r.right.line then
            process_line line (Some r.left.column) (Some r.right.column)
          else if start_line = r.left.line then
            process_line line (Some r.left.column) None
          else if start_line = r.right.line then
            process_line line None (Some r.right.column)
          else
            line
        in
        read_lines (start_line + 1) end_line (marked_line :: acc)
    in
    skip_lines (r.left.line - 1);
    let result = read_lines r.left.line r.right.line [] in
    close_in in_channel;
    Some result
  with e -> None

let read_region_with_markers r = read_region_with mark_line r
let read_region r = read_region_with extract_part r

(* generic parse error *)

exception ParseError of region * string
