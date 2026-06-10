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

(* Walk the file once with Uutf to record line offsets. [`ASCII] newline
   normalization recognises CR, LF and CRLF — matching the Motoko lexer
   (see [source_lexer.mll]). *)
let build_entry content =
  let dec = Uutf.decoder
    ~encoding:`UTF_8
    ~nln:(`ASCII (Uchar.of_int 0x0A))
    (`String content)
  in
  let starts = ref [0] in
  let rec loop prev_line =
    match Uutf.decode dec with
    | `End -> ()
    | `Uchar _ | `Malformed _ ->
      let cur_line = Uutf.decoder_line dec in
      if cur_line > prev_line then
        starts := Uutf.decoder_byte_count dec :: !starts;
      loop cur_line
    | `Await -> assert false
  in
  loop 1;
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

