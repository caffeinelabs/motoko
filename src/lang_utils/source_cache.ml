open Source

(* A loaded source file with everything needed to resolve a [pos] in O(1)
   (modulo Uutf walking the line for codepoint columns on non-ASCII files):
   - [line_starts.(N-1)] = byte offset of line N's first byte.
   - [ascii_only] short-circuits Uutf for the common case where codepoint
     column == byte column. *)
type entry = {
  content : string;
  line_starts : int array;
  ascii_only : bool;
}

type t = (string, entry option) Hashtbl.t

let create () : t = Hashtbl.create 16

(* The lexer treats [\n], [\r\n] and lone [\r] as line terminators (see
   [source_lexer.mll]); mirror that here so positions resolve consistently
   for old-Mac and CRLF sources. *)
let build_entry content =
  let len = String.length content in
  let starts = ref [0] in
  let ascii = ref true in
  String.iteri (fun i c ->
    if Char.code c >= 0x80 then ascii := false;
    let is_newline =
      c = '\n'
      || (c = '\r' && (i + 1 >= len || content.[i + 1] <> '\n'))
    in
    if is_newline then starts := (i + 1) :: !starts
  ) content;
  { content;
    line_starts = Array.of_list (List.rev !starts);
    ascii_only = !ascii }

let load (cache : t) path : entry option =
  match Hashtbl.find_opt cache path with
  | Some r -> r
  | None ->
    let r =
      try Some (build_entry (In_channel.with_open_bin path In_channel.input_all))
      with Sys_error _ -> None
    in
    Hashtbl.add cache path r; r

(* Resolve [pos] against a loaded entry. *)
let resolve_in e (pos : pos) : (int * int) option =
  if pos.line < 1 || pos.line > Array.length e.line_starts || pos.column < 0
  then None
  else
    let line_start = e.line_starts.(pos.line - 1) in
    let byte_off = line_start + pos.column in
    if byte_off > String.length e.content then None
    else if e.ascii_only then Some (pos.column, byte_off)
    else
      (* Count codepoints in [line_start, byte_off) without allocating a substring.
         Malformed sequences count as one codepoint. *)
      let codepoint_col = Uutf.String.fold_utf_8 ~pos:line_start ~len:pos.column
        (fun n _ _ -> n + 1) 0 e.content
      in
      Some (codepoint_col, byte_off)

let byte_offset cache (pos : pos) : int option =
  if pos.line <= 0 then None
  else
    match load cache pos.file with
    | Some e -> Option.map snd (resolve_in e pos)
    | None -> None

let codepoint_column cache (pos : pos) : int =
  if pos.line <= 0 then pos.column (* no_pos or binary [line = -1] *)
  else
    match load cache pos.file with
    | Some e ->
      (match resolve_in e pos with
       | Some (col, _) -> col
       | None -> pos.column)
    | None -> pos.column

let content cache path = Option.map (fun e -> e.content) (load cache path)

