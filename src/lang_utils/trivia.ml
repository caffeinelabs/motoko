open Source

type line_feed = LF | CRLF

type 'l trivia = Comment of string | Space of int | Tab of int | Line of 'l

type void = |

let absurd : void -> 'a = function _ -> .

let map_trivia : ('a -> 'b) -> 'a trivia -> 'b trivia =
 fun f -> function
  | Comment str -> Comment str
  | Space n -> Space n
  | Tab n -> Tab n
  | Line l -> Line (f l)

let string_of_line_feed = function LF -> "LF" | CRLF -> "CRLF"

let string_of_trivia : ('a -> string) -> 'a trivia -> string =
 fun f t ->
  match t with
  | Comment str -> str
  | Space n -> Printf.sprintf "Space(%d)" n
  | Tab n -> Printf.sprintf "Tab(%d)" n
  | Line l -> Printf.sprintf "Line(%s)" (f l)

let string_of_trivia_lf : line_feed trivia -> string =
  string_of_trivia string_of_line_feed

type trivia_info = {
  leading_trivia : line_feed trivia list;
  trailing_trivia : void trivia list;
}

let string_of_trivia_info (info : trivia_info) : string =
    let leading = List.map (string_of_trivia string_of_line_feed) info.leading_trivia |> String.concat ", " in
    let trailing = List.map (string_of_trivia (fun _ -> "")) info.trailing_trivia |> String.concat ", " in
    Printf.sprintf "Leading: [%s]; Trailing: [%s]" leading trailing

type pos = { line : int; column : int }

let pos_of_lexpos : Lexing.position -> pos =
 fun lexpos ->
  Lexing.{ line = lexpos.pos_lnum; column = lexpos.pos_cnum - lexpos.pos_bol }

module PosHash = struct
  type t = pos

  let equal i j = i = j

  let hash ({ line; column } : pos) = column lor 20 land line
end

module PosHashtbl = Hashtbl.Make (PosHash)

(* type triv_table = trivia_info IntHashtbl.t *)
type triv_table = trivia_info PosHashtbl.t

let empty_triv_table = PosHashtbl.create 0

let find_trivia triv_table (parser_pos : Source.region) : trivia_info =
  PosHashtbl.find triv_table
    { line = parser_pos.left.line; column = parser_pos.left.column }

let deprecated_of_trivia_info : trivia_info -> string option =
 fun info ->
  let lines =
    List.filter_map
      (function
        | Comment s -> (
            match Lib.String.chop_prefix "/// @deprecated" s with
            | Some "" -> Some ""
            | Some line_comment ->
                (* We expect a documentation line comment to start with a space
                 *  (which we remove here) *)
                Lib.String.chop_prefix " " line_comment
            | None -> None )
        | _ -> None)
      info.leading_trivia
  in
  if lines = [] then None else Some (String.concat "\n" lines)

type doc = LineComment of string | BlockComment of string

let docs_of_trivia_info (info : trivia_info) : doc list =
  List.filter_map
    (function
      | Comment s -> (
          match Lib.String.chop_prefix "///" s with
          | Some "" -> Some (LineComment "")
          | Some line_comment ->
              (* We expect a documentation line comment to start with a space
              *  (which we remove here) *)
              Lib.String.chop_prefix " " line_comment
              |> Option.map (fun c -> LineComment c)
          | None ->
              Option.bind
                (Lib.String.chop_prefix "/**" s)
                (Lib.String.chop_suffix "*/")
              |> Option.map String.trim
              |> Option.map (fun c -> BlockComment c))
      | _ -> None)
    info.leading_trivia
 
let doc_comment_of_trivia_info (info : trivia_info) : string option =
  let docs = docs_of_trivia_info info in
  let lines = List.map (function BlockComment s | LineComment s -> s) docs in
  if lines = [] then None else Some (String.concat "\n" lines)
