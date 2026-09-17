(*
This source file loads the RTS Wasm files via the environment
variables. This is for local development (e.g. inside `nix-shell`). The nix
build of `moc` will statically replace this file with one that just embeds
RTS Wasm files as static strings, to produce a fully self-contained `moc`
binary for distribution.
*)

let load_file env =
  match Sys.getenv_opt env with
  | Some filename ->
    let ic = open_in_bin filename in
    let n = in_channel_length ic in
    let s = Bytes.create n in
    really_input ic s 0 n;
    close_in ic;
    Bytes.to_string s
  | None ->
    Printf.eprintf "Environment variable %s not set. Please run moc via the bin/moc wrapper (which should be in your PATH in the nix-shell)." env;
    exit 1

let wasm_eop_release : string Lazy.t = lazy (load_file "MOC_EOP_RELEASE_RTS")
let wasm_eop_debug : string Lazy.t = lazy (load_file "MOC_EOP_DEBUG_RTS")
