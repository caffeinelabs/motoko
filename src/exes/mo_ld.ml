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
let mco_files : string list ref = ref []
let trace_import = ref ""

(* spike debugging: rewrite every `call <ic0.<trace_import>>` site into a call
   to a per-site thunk that ic0.traps with (ptr=0, len=site_id): the reported
   trap-message length identifies which function made the call at runtime. *)
let spike_trace_calls (em : Wasm_exts.CustomModule.extended_module) name =
  let open Wasm_exts.Ast in
  let open Wasm.Source in
  let nr x = x @@ no_region in
  let m = em.module_ in
  (* find the import indices of ic0.<name> and ic0.trap *)
  let fun_imports = List.filter (fun i ->
    match i.it.idesc.it with FuncImport _ -> true | _ -> false) m.imports in
  let idx_of nm =
    List.filteri (fun _ _ -> true) fun_imports |>
    List.mapi (fun i imp -> (i, imp)) |>
    List.filter_map (fun (i, imp) ->
      if imp.it.module_name = Lib.Utf8.decode "ic0" && imp.it.item_name = Lib.Utf8.decode nm
      then Some (Int32.of_int i) else None) in
  let targets = idx_of name in
  let traps = idx_of "trap" in
  let trap_idx = match traps with t :: _ -> t | [] -> failwith "no ic0.trap import" in
  Printf.eprintf "spike-trace: targets=%s trap=%ld\n"
    (String.concat "," (List.map Int32.to_string targets)) trap_idx;
  let n_fun_imports = Int32.of_int (List.length fun_imports) in
  let n_funcs = Int32.of_int (List.length m.funcs) in
  (* unit->i64 type for thunks: reuse the target's type? thunk must match the
     replaced callee's type: () -> i64 for stable64_size *)
  let thunk_type = Int32.of_int (List.length m.types) in
  let new_type = nr (Wasm_exts.Types.FuncType ([], [Wasm_exts.Types.I64Type])) in
  let site = ref 0 in
  let new_thunks = ref [] in
  let names = ref [] in
  let fresh_thunk caller_name =
    incr site;
    let k = !site in
    let fi = Int32.(add (add n_fun_imports n_funcs) (of_int (List.length !new_thunks))) in
    let store8 addr byte = [
      nr (Const (nr (Wasm_exts.Values.I64 (Int64.of_int addr))));
      nr (Const (nr (Wasm_exts.Values.I64 (Int64.of_int byte))));
      nr (Store Wasm_exts.Types.{ty = I64Type; align = 0; offset = 0L; sz = Some Pack8});
    ] in
    let body =
      store8 16 (0x30 + k / 10) @
      store8 17 (0x30 + k mod 10) @ [
      nr (Const (nr (Wasm_exts.Values.I64 16L)));
      nr (Const (nr (Wasm_exts.Values.I64 2L)));
      nr (Call (nr trap_idx));
      nr Unreachable;
    ] in
    new_thunks := !new_thunks @ [ nr { ftype = nr thunk_type; locals = []; body } ];
    names := !names @ [ (fi, Printf.sprintf "spike_trap_site_%d" k) ];
    Printf.eprintf "spike-trace: site %d = call from %s\n" k caller_name;
    fi in
  let fname fi =
    match List.assoc_opt fi em.name.function_names with
    | Some n -> n | None -> Printf.sprintf "func%ld" fi in
  let rec rewrite_instr caller i =
    match i.it with
    | Call v when List.mem v.it targets ->
      { i with it = Call (nr (fresh_thunk caller)) }
    | Block (bt, is) -> { i with it = Block (bt, List.map (rewrite_instr caller) is) }
    | Loop (bt, is) -> { i with it = Loop (bt, List.map (rewrite_instr caller) is) }
    | If (bt, is1, is2) ->
      { i with it = If (bt, List.map (rewrite_instr caller) is1, List.map (rewrite_instr caller) is2) }
    | _ -> i in
  let funcs = List.mapi (fun j f ->
    let caller = fname Int32.(add n_fun_imports (of_int j)) in
    { f with it = { f.it with body = List.map (rewrite_instr caller) f.it.body } }) m.funcs in
  { em with
    module_ = { m with
      funcs = funcs @ !new_thunks;
      types = m.types @ [ new_type ] };
    name = { em.name with function_names = em.name.function_names @ !names } }

let print_banner () =
  printf "%s\n" banner;
  exit 0

let usage_err s =
  eprintf "%s: %s\n" name s;
  eprintf "%s\n" usage;
  exit 1

let argspec =
[
  "-b", Arg.Set_string base_file, "<file> base file (e.g. output of moc --no-link)";
  "-l", Arg.Set_string lib_file, "<file> library file";
  "-o", Arg.Set_string out_file, "<file> output file";
  "-n", Arg.Set_string lib_name, "<name> library name (defaults to \"rts\")";
  "-mco", Arg.String (fun f -> mco_files := !mco_files @ [f]),
    "<file> (spike) merge a frozen migration object into the base before linking the library";
  "-spike-trace", Arg.Set_string trace_import,
    "<name> (spike debug) replace calls of ic0.<name> with per-site identifying traps";
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
    try
      let base =
        List.fold_left
          (fun b f -> LinkModule.link_mco b (decode_file f))
          base !mco_files
      in
      LinkModule.link base !lib_name lib
    with LinkModule.LinkError e ->
      Printf.eprintf "%s\n" e;
      exit 1
  in
  let linked =
    if !trace_import <> "" then spike_trace_calls linked !trace_import else linked
  in
  let (_map, wasm) = CustomModuleEncode.encode linked in
  write_file !out_file wasm

