open Source

(* A loaded source file with precomputed line offsets.
   [line_starts.(N-1)] = byte offset of line N's first byte. Resolving a
   [pos] is O(line_length): Uutf walks the line prefix to count codepoints. *)
type entry = {
  content : string;
  line_starts : int array;
}

type t = (string, entry option) Hashtbl.t

let create () : t = Hashtbl.create 16

(* Record line offsets. CR, LF and CRLF each end a line, matching the
   Motoko lexer (see [source_lexer.mll]). Not Uutf's newline normalization:
   it reports the new line at the CR of a CRLF, one byte early. *)
let build_entry content =
  let len = String.length content in
  let starts = ref [0] in
  content |> String.iteri (fun i c ->
    match c with
    | '\n' -> starts := (i + 1) :: !starts
    | '\r' when i + 1 >= len || content.[i + 1] <> '\n' ->
      starts := (i + 1) :: !starts
    | _ -> ());
  { content; line_starts = Array.of_list (List.rev !starts) }

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


(* Fresh cache per call: the file may change between calls (moc.js, VSCode). *)
let read_region_with process (r : region) =
  if r.left.line <= 0 then None
  else
    match load (create ()) r.left.file with
    | None -> None
    | Some e ->
      match resolve_in e r.left, resolve_in e r.right with
      | Some (_, start), Some (_, stop) when start <= stop ->
        Some (process e.content start stop)
      | _ -> None

let read_region = read_region_with (fun content start stop ->
  String.sub content start (stop - start))

(* The region's whole lines, with [**] around the region itself. *)
let read_region_with_markers = read_region_with (fun content start stop ->
  let is_break c = c = '\n' || c = '\r' in
  let rec line_start i =
    if i > 0 && not (is_break content.[i - 1]) then line_start (i - 1) else i in
  let rec line_end i =
    if i < String.length content && not (is_break content.[i]) then line_end (i + 1) else i in
  let ls = line_start start and le = line_end stop in
  String.concat "**" [
    String.sub content ls (start - ls);
    String.sub content start (stop - start);
    String.sub content stop (le - stop)])
