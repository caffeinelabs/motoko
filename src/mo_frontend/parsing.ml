open Mo_config
open Source

module P =
  MenhirLib.Printers.Make
    (Parser.MenhirInterpreter)
    (Printers)

(* Instantiate [ErrorReporting] for our parser. This requires
   providing a few functions -- see [CalcErrorReporting]. *)

module E =
  Menhir_error_reporting.Make
    (Parser.MenhirInterpreter)
    (Error_reporting)

(* Define a printer for explanations. We treat an explanation as if it
   were just an item: that is, we ignore the position information that
   is provided in the explanation. Indeed, this information is hard to
   show in text mode. *)

let uniq xs = List.fold_right (fun x ys -> if List.mem x ys then ys else x::ys) xs []

let abstract_symbols explanations =
  let symbols = List.sort Parser.MenhirInterpreter.compare_symbols
    (List.map (fun e -> List.hd (E.future e))  explanations) in
  let ss = List.map Printers.string_of_symbol symbols in
  String.concat "\n  " (uniq ss)

let abstract_future future =
  let ss = List.map Printers.string_of_symbol future in
  String.concat " " ss

let abstract_future_with_example future =
  let ss      = List.map Printers.string_of_symbol future |> String.concat " " in
  let example = List.map Printers.example_of_symbol future |> String.concat " " in
  if String.compare ss example != 0 then
    ss ^ " (e.g. '" ^ example ^ "')"
  else
    ss

let rec lex_compare_futures f1 f2 =
  match f1,f2 with
  | [], [] -> 0
  | s1::ss1,s2::ss2 ->
    (match Parser.MenhirInterpreter.compare_symbols s1 s2 with
     | 0 -> lex_compare_futures ss1 ss2
     | c -> c)
  | _ -> assert false

let compare_futures f1 f2 = match compare (List.length f1) (List.length f2) with
      | 0 -> lex_compare_futures f1 f2
      | c -> c

let abstract_futures explanations =
  let futures = List.sort compare_futures (List.map E.future explanations) in
  let ss = List.map abstract_future futures in
  String.concat "\n  " (uniq ss)

let abstract_futures_with_examples explanations =
  let futures = List.sort compare_futures (List.map E.future explanations) in
  let ss = List.map abstract_future_with_example futures in
  String.concat "\n  " (uniq ss)

let abstract_item item =
  P.print_item item;
  Printers.to_string()

let abstract_items explanations =
  let items = List.sort Parser.MenhirInterpreter.compare_items (List.map E.item explanations) in
  let ss = List.map abstract_item items in
  String.concat "  " (uniq ss)

type error_detail = int

exception Error of string * Lexing.position * Lexing.position

(* The lexbuf is a 1024 byte wide window, we need to compute offsets before
   accessing it, because token positions are absolute to the whole input *)
let slice_lexeme lexbuf i1 i2 =
  let open Lexing in
  let offset = i1.pos_cnum - lexbuf.lex_abs_pos in
  let len = i2.pos_cnum - i1.pos_cnum in
  if offset < 0 || len < 0
  then "<unknown>" (* Too rare to care *)
  else Bytes.sub_string lexbuf.lex_buffer offset len

module I = Parser.MenhirInterpreter

(* For debug *)
(* module RecoveryTracer = MenhirRecoveryLib.MakePrinter ( *)
(*   struct *)
(*     module I = Parser.MenhirInterpreter *)
(*     let print s = Printf.eprintf "%s" s *)
(*     let print_symbol s = print (Printers.string_of_symbol s) *)
(*     let print_element = None *)
(*     let print_token t = print (Source_token.string_of_parser_token t) *)
(*   end) *)

module RecoveryTracer = MenhirRecoveryLib.DummyPrinter (I)

module RecoveryConfig = struct
  include Recover_parser

  (* Adapt [default_value region] to MenhirRecoverLib interface ([default_value loc]) *)
  let default_value (loc: Custom_compiler_libs.Location.t) sym =
      let open Custom_compiler_libs.Location in
      let open Lexing in
      let open Source in
      let file = loc.loc_start.pos_fname in
      let region_loc : region = {
        left  : pos = {file; line = loc.loc_start.pos_lnum; column = loc.loc_start.pos_bol};
        right : pos = {file; line = loc.loc_end.pos_lnum; column = loc.loc_end.pos_bol};
      } in
      default_value region_loc sym (* [default_value] is included from Recover_parser *)

  let guide _ = false
  let use_indentation_heuristic = false
  let is_eof  = function Parser.EOF -> true | _ -> false
end

module R = MenhirRecoveryLib.Make (Parser.MenhirInterpreter) (RecoveryConfig) (RecoveryTracer)

let handle_error lexbuf error_detail message_store (start, end_) explanations =
  let at =
        {left = Lexer.convert_pos start; right = Lexer.convert_pos end_}
  in
  let lexeme = slice_lexeme lexbuf start end_ in
  let token =
    if lexeme = "" then "end of input" else
      "token '" ^ String.escaped lexeme ^ "'"
  in
  let msg =
    match error_detail with
    | 1 ->
      Printf.sprintf
        "unexpected %s, expected one of token or <phrase>:\n  %s"
        token (abstract_symbols explanations)
    | 2 ->
      Printf.sprintf
        "unexpected %s, expected one of token or <phrase> sequence:\n  %s"
        token (abstract_futures explanations)
    | 3 ->
      Printf.sprintf
        "unexpected %s in position marked . of partially parsed item(s):\n%s"
        token (abstract_items explanations)
    | 4 ->
      Printf.sprintf
        "unexpected %s, expected one of token or <phrase> sequence:\n  %s"
        token (abstract_futures_with_examples explanations)
    | _ ->
      Printf.sprintf "unexpected %s" token
  in
  Diag.add_msg message_store (Diag.error_message at "M0001" "syntax" msg)

(* We drive the parser in the usual way, but records the last [InputNeeded]
   checkpoint. If a syntax error is detected, we go back to this checkpoint
   and analyze it in order to produce a meaningful diagnostic. *)

let parse ?(recovery = false) mode error_detail start lexer lexbuf =
  Diag.with_message_store ~allow_errors:recovery (fun m ->
    Parser_lib.msg_store := Some m;
    Parser_lib.mode := Some mode;
    let save_error (inputneeded_cp : 'a I.checkpoint) (fail_cp : 'a I.checkpoint) : unit =
    (* The parser signals a syntax error. Note the position of the
         problematic token, which is useful. Then, go back to the
         last [InputNeeded] checkpoint and investigate. *)
      match fail_cp with
      | I.HandlingError env ->
        let (startp, _) as positions = I.positions env in
        let explanations = E.investigate startp inputneeded_cp in
        handle_error lexbuf error_detail m positions explanations
      | _ -> assert false
    in
    let fail cp = None in
    let save_error_and_fail cp1 cp2 = save_error cp1 cp2; fail cp2 in
    let succ e =  Some e in
    if recovery || (!Flags.error_recovery) then
      R.loop_handle_recover succ fail save_error lexer start
    else
      I.loop_handle_undo succ save_error_and_fail lexer start
  )

