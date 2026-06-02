open Mo_config
module G = Grace
module GD = Grace.Diagnostic
open Source

type error_code = string
type severity = Warning | Error | Info
type priority = Primary | Secondary
type span = {
  prio : priority;
  at_span : region;
  label : string;
}
type edit = {
  at_edit : region;
  suggested_replacement : string;
}
type message = {
  sev : severity;
  code : error_code;
  at : region;
  cat : string;
  text : string;
  spans : span list;
  notes: string list;
  edits : edit list;
}
type messages = message list

let info_message at cat ?(spans = []) ?(notes = []) ?(edits = []) text =
  {sev = Info; code = ""; at; cat; text; spans; notes; edits}
let warning_message at code cat ?(spans = []) ?(notes = []) ?(edits = []) text =
  {sev = Warning; code; at; cat; text; spans; notes; edits}
let error_message at code cat ?(spans = []) ?(notes = []) ?(edits = []) text =
  {sev = Error; code; at; cat; text; spans; notes; edits}

type 'a result = ('a * messages, messages) Stdlib.result

let return x = Ok (x, [])

let info at cat text = Ok ((), [info_message at cat text])
let warn at code cat text = Ok ((), [warning_message at code cat text])
let error at code cat text = Stdlib.Error [error_message at code cat text]

let map f = function
  | Stdlib.Error msgs -> Stdlib.Error msgs
  | Ok (x, msgs) -> Ok (f x, msgs)

let bind x f = match x with
  | Stdlib.Error msgs -> Stdlib.Error msgs
  | Ok (y, msgs1) -> match f y with
    | Ok (z, msgs2) -> Ok (z, msgs1 @ msgs2)
    | Stdlib.Error msgs2 -> Error (msgs1 @ msgs2)

let finally f r = f (); r

module Syntax = struct
  let (let*) = bind
end

let rec traverse : ('a -> 'b result) -> 'a list -> 'b list result = fun f -> function
  | [] -> return []
  | x :: xs -> bind (f x) (fun y -> map (fun ys -> y :: ys) (traverse f xs))

let rec traverse_ : ('a -> unit result) -> 'a list -> unit result = fun f -> function
  | [] -> return ()
  | x :: xs -> bind (f x) (fun () -> traverse_ f xs)

let rec fold : ('a -> 'b -> 'a result) -> 'a -> 'b list -> 'a result = fun f acc -> function
  | [] -> return acc
  | x :: xs -> bind (f acc x) (fun y -> fold f y xs)

type msg_store = messages ref
let add_msg s m =
  if m.sev = Warning && Flags.is_warning_disabled m.code then () else
  s := m :: !s
let add_msgs s ms = List.iter (add_msg s) (List.rev ms)
let get_msgs s = List.rev !s

let has_errors : messages -> bool =
  List.exists (fun msg -> msg.sev == Error)

(** A cache from file path to its (lazily loaded) content, used while rendering
    a single diagnostic to convert byte columns to codepoint columns without
    re-reading files. [None] means the file could not be read. *)
type content_cache = (string, string option) Hashtbl.t

let make_content_cache () : content_cache = Hashtbl.create 1

let load_content (cache : content_cache) path : string option =
  match Hashtbl.find_opt cache path with
  | Some r -> r
  | None ->
    let r =
      try Some (In_channel.with_open_bin path In_channel.input_all)
      with _ -> None
    in
    Hashtbl.add cache path r; r

(** Convert a 0-based byte column on [pos.line] to a 0-based codepoint column.
    Falls back to the byte column if the file is unavailable or the position
    lies past the available content. *)
let byte_col_to_codepoint_col content (pos : pos) : int =
  let len = String.length content in
  let line_start = ref 0 in
  let cur_line = ref 1 in
  let i = ref 0 in
  while !cur_line < pos.line && !i < len do
    if content.[!i] = '\n' then begin
      incr cur_line;
      line_start := !i + 1
    end;
    incr i
  done;
  if !cur_line < pos.line then pos.column
  else
    let target = !line_start + pos.column in
    let p = ref !line_start in
    let codepoints = ref 0 in
    while !p < target && !p < len do
      let b = Char.code content.[!p] in
      let step =
        if b < 0x80 then 1
        else if b < 0xc0 then 1 (* malformed lead byte; advance one byte defensively *)
        else if b < 0xe0 then 2
        else if b < 0xf0 then 3
        else 4
      in
      p := !p + step;
      incr codepoints
    done;
    !codepoints

let display_column (cache : content_cache) (pos : pos) : int =
  if pos.line <= 0 then pos.column (* no_pos or binary [line = -1]; column is opaque *)
  else
    match load_content cache pos.file with
    | Some content -> byte_col_to_codepoint_col content pos
    | None -> pos.column

let string_of_pos_display cache pos =
  if pos.line = -1 then
    Printf.sprintf "0x%x" pos.column
  else
    string_of_int pos.line ^ "." ^ string_of_int (display_column cache pos + 1)

let string_of_region_display cache r =
  if r.left.file = "" then "(unknown location)" else
  r.left.file ^ ":" ^ string_of_pos_display cache r.left ^
  (if r.right = r.left then "" else "-" ^ string_of_pos_display cache r.right)

let string_of_message msg =
  let cache = make_content_cache () in
  let code = match msg.sev, msg.code with
    | Info, _ -> ""
    | _, "" -> ""
    | _, code -> Printf.sprintf " [%s]" code in
  let label = match msg.sev with
    | Error -> Printf.sprintf "%s error" msg.cat
    | Warning -> "warning"
    | Info -> "info" in
  let spans =
    let primary_spans = List.filter (fun span -> span.prio = Primary) msg.spans in
    if primary_spans <> [] then
      "\n" ^ String.concat "\n" (List.map (fun (span : span) -> span.label) primary_spans)
    else "" in
  let notes =
    if msg.notes <> [] then
      "\n" ^ String.concat "\n" (List.map (fun note -> "note: " ^ note) msg.notes)
    else "" in
  Printf.sprintf "%s: %s%s, %s%s%s\n" (string_of_region_display cache msg.at) label code msg.text spans notes

(** Converts a line/column based position to a byte offset.

    NOTE(Christoph): This is rather inefficient. If at some point this needs to be sped up,
    we could maintain a datastructure like https://crates.io/crates/line-index
*)
let pos_to_byte content pos =
  let line_start = ref (-1) in
  for _ = 1 to pos.line - 1 do
    let prev = !line_start in
    line_start := String.index_from content (prev + 1) '\n';
  done;
  !line_start + pos.column + 1

let ensure_primary_span msg =
  if List.exists (fun span -> span.prio = Primary) msg.spans
  then msg.spans
  else { prio = Primary; at_span = msg.at; label = "" } :: msg.spans

let fancy_of_message (msg : message) =
  if is_no_region msg.at then string_of_message msg else
  let path = msg.at.left.file in
  let content = In_channel.with_open_bin path In_channel.input_all in
  let file = G.Source.{ name = Some path; content } in
  let source : G.Source.t = `String file in
  let range r =
    G.Range.create ~source
      (G.Byte_index.of_int (pos_to_byte content r.left))
      (G.Byte_index.of_int (pos_to_byte content r.right))
  in
  let mk_span span =
    let priority = match span.prio with
      | Primary -> GD.Priority.Primary
      | Secondary -> GD.Priority.Secondary in
    GD.Label.createf ~range:(range span.at_span) ~priority "%s" span.label in
  let labels = List.map mk_span (ensure_primary_span msg) in
  let source_text r =
    let start = pos_to_byte content r.left in
    let stop = pos_to_byte content r.right in
    String.sub content start (stop - start)
    |> Lib.String.strip_control_chars
    |> String.trim
  in
  let edit_note edit =
    (* Future work: merge the replacements and display a diff *)
    let original = source_text edit.at_edit in
    if edit.suggested_replacement = "" then
      GD.Message.createf "help: remove `%s`" original
    else if original = "" then
      GD.Message.createf "help: insert `%s`" edit.suggested_replacement
    else
      GD.Message.createf "help: replace `%s` with `%s`" original edit.suggested_replacement
  in
  let severity = match msg.sev with
    | Error -> GD.Severity.Error
    | Warning -> GD.Severity.Warning
    | Info -> GD.Severity.Help in
  let notes =
    List.map (GD.Message.createf "note: %s") msg.notes
    @ List.map edit_note msg.edits in
  let code = if msg.code = "" then None else Some msg.code in
  let diag = GD.createf ~labels ~notes ?code severity "%s" msg.text in
  Format.asprintf "%a@." Grace_ansi_renderer.(pp_diagnostic ~config:Config.default ~code_to_string: Fun.id) diag

let string_of_severity (sev : severity) = match sev with
  | Error -> "error"
  | Warning -> "warning"
  | Info -> "info"

let json_span cache ?prio ?label ?suggested_replacement r =
  let { file; line = line_start; _ } = r.left in
  let { line = line_end; _ } = r.right in
  let column_start = display_column cache r.left in
  let column_end = display_column cache r.right in
  `Assoc [
    "file", `String file;
    "line_start", `Int line_start;
    "column_start", `Int (column_start + 1);
    "line_end", `Int line_end;
    "column_end", `Int (column_end + 1);
    "is_primary", `Bool (prio = Some Primary);
    "label", (match label with None -> `Null | Some label -> `String label);
    "suggested_replacement", (match suggested_replacement with None -> `Null | Some s -> `String s);
    "suggestion_applicability", (match suggested_replacement with
      | None -> `Null
      | Some _ -> `String "MachineApplicable");
  ]

(* Keep in sync with [design/JSON-Diagnostics.md] *)
let json_string_of_message msg =
  let cache = make_content_cache () in
  let span_jsons = ensure_primary_span msg
    |> List.map (fun { prio; at_span; label } -> json_span cache ~prio ~label at_span) in
  let edit_jsons = msg.edits |>
    List.map (fun { at_edit; suggested_replacement } -> json_span cache ~suggested_replacement at_edit) in
  let json = `Assoc [
    "message", `String msg.text;
    "code", `String msg.code;
    "level", `String (string_of_severity msg.sev);
    "spans", `List (span_jsons @ edit_jsons);
    "notes", `List (List.map (fun n -> `String n) msg.notes);
  ] in
  Yojson.Basic.to_string json

let is_warning_as_error msg =
  msg.sev = Warning && Flags.get_warning_level msg.code = Flags.Error

let is_treated_as_error msg =
  msg.sev = Error || is_warning_as_error msg

let normalize_severity msg =
  if is_warning_as_error msg
  then { msg with sev = Error }
  else msg

let print_message msg =
  let msg = normalize_severity msg in
  if msg.sev <> Error && not !Flags.print_warnings
  then ()
  else match !Flags.error_format with
  | Flags.Plain -> Printf.eprintf "%s%!" (string_of_message msg)
  | Flags.Human -> Printf.eprintf "%s%!" (fancy_of_message msg)
  | Flags.Json -> Printf.printf "%s\n%!" (json_string_of_message msg)

let print_messages = List.iter print_message

let is_error_free (ms: msg_store) = not (has_errors (get_msgs ms))

let with_message_store ?(allow_errors = false) f =
  let s = ref [] in
  let r = f s in
  let msgs = get_msgs s in
  match r with
  | Some x when allow_errors || not (has_errors msgs) -> Ok (x, msgs)
  | _ -> Error msgs

let flush_messages : 'a result -> 'a option = function
  | Stdlib.Error msgs ->
    print_messages msgs;
    None
  | Ok (x, msgs) ->
    print_messages msgs;
    if (!Flags.warnings_are_errors && msgs <> [])
      || List.exists is_warning_as_error msgs
    then None
    else Some x

let run r = match flush_messages r with
  | None -> exit 1
  | Some x -> x
