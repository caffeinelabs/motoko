open Mo_config

type error_code = string
type severity = Warning | Error | Info
type message = {
  sev : severity;
  code : error_code;
  at : Source.region;
  cat : string;
  text : string
}
type messages = message list

let info_message at cat text = {sev = Info; code = ""; at; cat; text}
let warning_message at code cat text = {sev = Warning; code; at; cat; text}
let error_message at code cat text = {sev = Error; code; at; cat; text}

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

let string_of_message msg =
  let code = match msg.sev, msg.code with
    | Info, _ -> ""
    | _, "" -> ""
    | _, code -> Printf.sprintf " [%s]" code in
  let label = match msg.sev with
    | Error -> Printf.sprintf "%s error" msg.cat
    | Warning -> "warning"
    | Info -> "info" in
  let src = if !Flags.print_source_on_error then
    match Source.read_region_with_markers msg.at with
    | Some(src) -> Printf.sprintf "> %s\n\n" src
    | None -> ""
  else "" in
  Printf.sprintf "%s: %s%s, %s\n%s" (Source.string_of_region msg.at) label code msg.text src

let string_of_severity (sev : severity) = match sev with
  | Error -> "error"
  | Warning -> "warning"
  | Info -> "info"

(* Keep in sync with [design/JSON-Diagnostics.md] *)
let json_string_of_message msg =
  let at = msg.at in
  let { Source.file; line = line_start; column = column_start } = at.Source.left in
  let { Source.line = line_end; column = column_end; _ } = at.Source.right in
  let span = `Assoc [
    "file", `String file;
    "line_start", `Int line_start;
    "column_start", `Int (column_start + 1);
    "line_end", `Int line_end;
    "column_end", `Int (column_end + 1);
  ] in
  let json = `Assoc [
    "message", `String msg.text;
    "code", `String msg.code;
    "level", `String (string_of_severity msg.sev);
    "spans", `List [span];
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
