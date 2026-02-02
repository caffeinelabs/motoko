(** Exception raised instead of calling exit in JS mode *)
exception Exit of string

(** Shadow stdlib's exit to raise an exception in JS mode instead of terminating *)
let exit code =
  if !Flags.ocaml_js then
    raise (Exit (Printf.sprintf "Fatal error (exit %d)" code))
  else
    Stdlib.exit code

(** Print error message and exit. In JS mode, raises Exit with the message *)
let fail fmt =
  Printf.ksprintf (fun msg ->
    if !Flags.ocaml_js then
      raise (Exit msg)
    else begin
      Printf.eprintf "%s" msg;
      Stdlib.exit 1
    end) fmt
