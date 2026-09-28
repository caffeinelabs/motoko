open Linking
open Wasm_exts
open Printf

let name = "mo-ld"
let version = "0.1"
let banner = "Motoko " ^ version ^ " linker"
let usage = "Usage: " ^ name ^ " -b base.wasm -l shared.wasm -o out.wasm"

(* Argument handling *)

let base_file = ref ""
let lib_file = ref ""
let lib_name = ref "rts"
let out_file = ref ""

let print_banner () =
  printf "%s\n" banner;
  exit 0

let usage_err s =
  eprintf "%s: %s\n" name s;
  eprintf "%s\n" usage;
  exit 1

let argspec =
[
  "-b", Arg.Set_string base_file, "<file> base file";
  "-l", Arg.Set_string lib_file, "<file> library file";
  "-o", Arg.Set_string out_file, "<file> output file";
  "-n", Arg.Set_string lib_name, "<name> library name (defaults to \"rts\")";
  "--version", Arg.Unit print_banner, " show version";
]

(* IO *)

let load_file f =
  let ic = open_in_bin f in
  let n = in_channel_length ic in
  let s = Bytes.create n in
  really_input ic s 0 n;
  close_in ic;
  Bytes.to_string s

let decode_file f =
  let wasm = load_file f in
  CustomModuleDecode.decode "f" wasm

let write_file f s =
  let oc_ = open_out f in
  output_string oc_ s;
  close_out oc_

(* Main *)
let () =
  if Array.length Sys.argv = 1 then print_banner ();
  Arg.parse argspec (fun _ -> usage_err "no arguments expected") usage;
  if !base_file = "" then usage_err "no base file specified";
  if !lib_file = "" then usage_err "no library file specified";
  if !out_file = "" then usage_err "no output file specified";

  Mo_config.Flags.debug_info := true; (* linking mode: preserve debug info *)

  let base = decode_file !base_file in
  let lib = decode_file !lib_file in
  let linked =
    try LinkModule.link base !lib_name lib
    with LinkModule.LinkError e ->
      Printf.eprintf "%s\n" e;
      exit 1
  in
  let (_map, wasm) = CustomModuleEncode.encode linked in
  write_file !out_file wasm

