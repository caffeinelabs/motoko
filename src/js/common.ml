open Wasm_exts
open Source
open Mo_config

module Js = Js_of_ocaml.Js
module Sys_js = Js_of_ocaml.Sys_js

(** Extra flags from the [moc] CLI *)
let moc_args = Mo_args.inclusion_args
  @ Mo_args.warning_args
  @ Mo_args.error_args
  @ Mo_args.ai_args
  @ Mo_args.persistent_actors_args

let position_of_pos pos =
  object%js
    (* The LSP spec requires zero-based positions *)
    val line = if pos.line > 0 then pos.line - 1 else 0
    val character = pos.column
  end

let range_of_region at =
  object%js
    val start = position_of_pos at.left
    val _end = position_of_pos at.right
  end

let diagnostics_of_msg (msg : Diag.message) =
  Diag.(object%js
    val source = Js.string msg.at.left.file
    val severity = if Diag.is_treated_as_error msg then 1 else 2
    val range = range_of_region msg.at
    val code = Js.string msg.code
    val category = Js.string msg.cat
    val message = Js.string msg.text
  end)

let diagnostics_of_msgs (msgs : Diag.message list) =
  Array.of_list (List.map diagnostics_of_msg msgs)

let rec js_of_sexpr (sexpr : Wasm.Sexpr.sexpr) : Js.Unsafe.any =
  (* generate a JSON-serializable value tree from an s-expression *)
  match sexpr with
| Wasm.Sexpr.Node (head, inner) ->
  Js.Unsafe.coerce (object%js
    val name = Js.string head
    val args = inner |> List.map js_of_sexpr |> Array.of_list |> Js.array |> Js.some
  end)
| Wasm.Sexpr.Atom s ->
  Js.Unsafe.coerce (Js.string s)

let js_result (result : 'a Diag.result) (wrap_code: 'a -> 'b) =
  match result with
  | Ok (code, msgs) ->
     object%js
       val diagnostics = Js.array (diagnostics_of_msgs msgs)
       val code = wrap_code code
     end
  | Error msgs ->
     object%js
       val diagnostics = Js.array (diagnostics_of_msgs msgs)
       val code = Js.null
     end

let js_version = Js.string Source_id.id

let js_check source =
  Mo_types.Cons.session (fun _ ->
    js_result
      (Pipeline.check_files ~enable_recovery:true [Js.to_string source])
      (fun _ -> Js.null))

let parse_extra_flags tokens =
  let argv = Array.append [|"mocjs"|] tokens in
  let current = ref 0 in
  let usage = "moc extra flags" in
  Arg.parse_argv ~current argv moc_args (fun _ -> ()) usage 

let js_set_extra_flags flags =
  let tokens = flags |> Js.to_array |> Array.map Js.to_string in
  parse_extra_flags tokens

let js_set_run_step_limit limit =
  Mo_interpreter.Interpret.step_limit := limit

let js_run list source =
  Mo_types.Cons.session (fun _ ->
    let list = Js.to_array list |> Array.to_list |> List.map Js.to_string in
    match Pipeline.run_stdin_from_file list (Js.to_string source) with
    | Some v ->
      object%js
        (* TODO: val value = js_value v *)
        val error = Js.null
      end
    | None ->
      object%js
        val error = Js.some (object%js
          (* empty for compatibility with returning a message, type, location, etc. in the future *)
        end)
      end)

let js_candid source =
  Mo_types.Cons.session (fun _ ->
    js_result (Pipeline.generate_idl [Js.to_string source])
      (fun prog ->
        let open Idllib in
        let module WithComments = Arrange_idl.Make(struct let trivia = Some prog.note.Syntax.trivia end) in
        let code = WithComments.string_of_prog prog in
        Js.some (Js.string code)))

let js_stable_compatible pre post =
  js_result (Pipeline.stable_compatible (Js.to_string pre) (Js.to_string post)) (fun _ -> Js.null)

let js_compile_wasm mode source =
  let source = Js.to_string source in
  let mode =
    match Js.to_string mode with
    | "wasi" -> Flags.WASIMode
    | "ic" -> Flags.ICMode
    | _ -> raise (Invalid_argument "js_compile_with: Unexpected mode")
  in
  Mo_types.Cons.session (fun _ -> js_result (Pipeline.compile_files mode true [source])
    (fun (idl_prog, m) ->
      let open CustomModule in
      let sig_ = match m.motoko.stable_types with
        | Some (_, txt) -> Js.some (Js.string txt)
        | _ -> Js.null in
      let candid = Idllib.Arrange_idl.string_of_prog idl_prog in
      let (_, wasm) = CustomModuleEncode.encode m in
      let constructor = Js.Unsafe.global##._Uint8Array in
      let code = constructor##from
        (object%js val length = String.length wasm end)
        (Js.wrap_callback (fun _v k -> Char.code wasm.[k])) in
      Js.some (object%js
        val wasm = code
        val candid = Js.string candid
        val stable = sig_
      end)))

let js_parse_candid s =
  let parse_result = Idllib.Pipeline.parse_string (Js.to_string s) in
  js_result parse_result (fun (prog, _) ->
    Js.some (js_of_sexpr (Idllib.Arrange_idl.prog prog)))

let js_parse_motoko enable_recovery s =
  let main_file = "" in
  let parse_fn = if Js.Opt.get enable_recovery (fun () -> false)
    then Pipeline.parse_string_with_recovery
    else Pipeline.parse_string
  in
  let parse_result = parse_fn main_file (Js.to_string s) in
  js_result parse_result (fun (prog, _) ->
    let open Mo_def in
    let module Arrange = Astjs.Make (struct
      let include_sources = true
      let include_type_rep = Arrange.Without_type_rep
      let include_types = false
      let include_docs = Some prog.note.Syntax.trivia
      let include_parenthetical = false
      let main_file = Some main_file
    end)
    in Js.some (Arrange.prog_js prog))

let js_parse_motoko_with_deps enable_recovery path s =
  let main_file = Js.to_string path in
  let s = Js.to_string s in
  let parse_fn = if Js.Opt.get enable_recovery (fun () -> false)
    then Pipeline.parse_string_with_recovery
    else Pipeline.parse_string
  in
  let prog_and_deps_result =
    let open Diag.Syntax in
    let* prog, _ = parse_fn main_file s in
    let* deps =
      Pipeline.ResolveImport.resolve (Pipeline.resolve_flags None) prog main_file
    in
    Diag.return (prog, deps)
  in
  js_result prog_and_deps_result (fun (prog, deps) ->
    let open Mo_def in
    let module Arrange = Astjs.Make (struct
      let include_sources = true
      let include_type_rep = Arrange.Without_type_rep
      let include_types = false
      let include_docs = Some prog.note.Syntax.trivia
      let include_parenthetical = false
      let main_file = Some main_file
    end) in
    Js.some (
      object%js
        val ast = Arrange.prog_js prog
        val immediateImports =
          deps
          |> List.map (fun dep -> Js.string (Pipeline.resolved_import_name dep))
          |> Array.of_list
          |> Js.array
      end))

module Map_conversion (Map : Map.S) = struct
  let from_js
    (type data)
    (map : _ Js.t)
    (from_key : 'k Js.t -> Map.key)
    (from_data : 'd Js.t -> data)
    : data Map.t
    =
    let result = ref Map.empty in
    let callback =
      (* [forEach] in JS gives value, key, and map, in this order. *)
      Js.wrap_callback (fun v k _m ->
          let k = from_key k in
          let v = from_data v in
          result := Map.add k v !result)
    in
    ignore (Js.Unsafe.meth_call map "forEach" [|Js.Unsafe.inject callback|]);
    !result

  let to_js
    (type data)
    (map : data Map.t)
    (from_key : Map.key -> Js.Unsafe.top Js.t)
    (from_data : data -> Js.Unsafe.top Js.t)
    : _ Js.t
    =
    let js_map = Js.Unsafe.new_obj (Js.Unsafe.pure_js_expr "Map") [||] in
    Map.iter
      (fun k v ->
         ignore (Js.Unsafe.meth_call js_map "set" [| from_key k; from_data v |]))
      map;
    js_map

  let from_ocaml = to_js
  let to_ocaml = from_js
end

(* Scope cache conversion between JS Maps and OCaml Type.Env.
   The data of the map has TypeScript [type Scope = unknown], and such
   scopes are always produced by the compiler. The language server takes
   scopes as outputs and gives them as inputs without touching them at
   all. Hence, the use of [Obj.magic] is legitimate here. *)
module ScopeCacheConversion = Map_conversion (Mo_types.Type.Env)

let scope_cache_from_js js_cache =
  ScopeCacheConversion.from_js js_cache Js.to_string Obj.magic

let scope_cache_to_js cache =
  ScopeCacheConversion.to_js
    cache
    (fun k -> Js.Unsafe.inject (Js.string k))
    Obj.magic

let load_progs_cached_js
    ~enable_type_recovery
    parse_fn
    paths
    scope_cache =
  Mo_types.Cons.session (fun () ->
    Pipeline.load_progs_cached ~enable_type_recovery
      parse_fn paths Pipeline.initial_stat_env scope_cache)

let js_parse_motoko_typed_with_scope_cache_impl enable_recovery paths scope_cache =
  let paths = paths |> Js.to_array |> Array.to_list |> List.map Js.to_string in
  let scope_cache =
    Js.Opt.case scope_cache
      (fun () -> Mo_types.Type.Env.empty)
      scope_cache_from_js
  in
  let recovery_enabled = Js.Opt.get enable_recovery (fun () -> false) in
  let parse_fn = if recovery_enabled
    then Pipeline.parse_file_with_recovery
    else Pipeline.parse_file
  in
  let load_result =
    load_progs_cached_js ~enable_type_recovery:recovery_enabled
      parse_fn paths scope_cache
  in
  match load_result with
  | Ok ((_libs, progs, _senv, scope_cache), msgs) ->
    let progs =
      progs |> List.map (fun (prog, immediate_imports, sscope) ->
        let open Mo_def in
        let module Arrange = Astjs.Make (struct
          let include_sources = true
          let include_type_rep = Arrange.With_type_rep (Some sscope.Mo_frontend.Scope.fld_src_env)
          let include_types = true
          let include_docs = Some prog.note.Syntax.trivia
          let include_parenthetical = false
          let main_file = Some prog.at.left.file
        end)
        in
        ( Arrange.prog_js prog
        (* , js_of_sexpr (Arrange_sources_types.typ typ) *)
        , immediate_imports |> List.map Js.string |> Array.of_list |> Js.array )
      ) |> Array.of_list
    in
    Ok ((progs, scope_cache_to_js scope_cache), msgs)
  | Error msgs -> Error msgs

let js_parse_motoko_typed_with_scope_cache enable_recovery paths scope_cache =
  let result =
    js_parse_motoko_typed_with_scope_cache_impl enable_recovery paths (Js.Opt.return scope_cache)
  in
  js_result result (fun (progs, scope_cache) ->
    let progs =
      progs |> Array.map (fun (ast, immediate_imports) ->
        object%js
          val ast = ast
          (* val typ = typ *)
          val immediateImports = immediate_imports
        end)
      |> Js.array
    in
    Js.some (Js.array [| Js.Unsafe.inject progs; Js.Unsafe.inject scope_cache |]))

let js_parse_motoko_typed paths =
  let result = js_parse_motoko_typed_with_scope_cache_impl Js.Opt.empty paths Js.Opt.empty in
  js_result result (fun (progs, _scope_cache) ->
    let progs =
      progs |> Array.map (fun (ast, _immediate_imports) ->
        object%js
          val ast = ast
          (* val typ = typ *)
        end)
      |> Js.array
    in
    Js.some (Js.Unsafe.inject progs))

let js_check_with_scope_cache source scope_cache =
  let load_result =
    load_progs_cached_js
      ~enable_type_recovery:true
      Pipeline.parse_file_with_recovery
      [Js.to_string source]
      (scope_cache_from_js scope_cache)
  in
  let msgs, scope_cache_js = match load_result with
    | Ok ((_libs, _progs, _senv, scope_cache), msgs) ->
      msgs, Js.some (scope_cache_to_js scope_cache)
    | Error msgs ->
      msgs, Js.null
  in
  object%js
    val diagnostics = Js.array (diagnostics_of_msgs msgs)
    val scopeCache = scope_cache_js
  end

let js_save_file filename content =
  let filename = Js.to_string filename in
  let content = Js.to_string content in
  try Sys_js.create_file ~name:filename ~content:content
  with _ -> Sys_js.update_file ~name:filename ~content:content

let js_remove_file filename = Sys.remove (Js.to_string filename)
let js_rename_file oldpath newpath = Sys.rename (Js.to_string oldpath) (Js.to_string newpath)
let js_read_file path = Sys_js.read_file ~name:(Js.to_string path) |> Js.string
let js_read_dir path = Sys.readdir (Js.to_string path) |> Array.map Js.string |> Js.array
let stdout_buffer = Buffer.create(1000)
let stderr_buffer = Buffer.create(1000)

let wrap_output f =
  let result = f () in
  let stdout_result = Buffer.contents stdout_buffer in
  let stderr_result = Buffer.contents stderr_buffer in
  Buffer.clear stdout_buffer;
  Buffer.clear stderr_buffer;
  object%js
    val stdout = Js.bytestring stdout_result
    val stderr = Js.bytestring stderr_result
    val result = result
  end

let print_deps file =
  let _ = Pipeline.print_deps (Js.to_string file) in
  let stdout_result = Buffer.contents stdout_buffer in
  Buffer.clear stdout_buffer;
  Js.bytestring stdout_result

let add_package package dir =
  let libs = Flags.package_urls in
  libs := Flags.M.add (Js.to_string package) (Js.to_string dir) !libs

let clear_package () = Flags.package_urls := Flags.M.empty

let set_candid_path path = Flags.actor_idl_path := Some (Js.to_string path)

let set_actor_aliases entries =
  let entries = Array.map (fun kv ->
                    let kv = Js.to_array kv in
                    Js.to_string (Array.get kv 0), Js.to_string (Array.get kv 1)) (Js.to_array entries) in
  let aliases = Flags.actor_aliases in
  aliases := Flags.M.of_seq (Array.to_seq entries)

let set_public_metadata entries =
  let entries = Array.map Js.to_string (Js.to_array entries) in
  Flags.public_metadata_names := Array.to_list entries

let gc_flags option =
  match Js.to_string option with
  | "force" -> Flags.force_gc := true
  | "scheduling" -> Flags.force_gc := false
  | "copying" -> Flags.gc_strategy := Mo_config.Flags.Copying
  | "marking" -> Flags.gc_strategy := Mo_config.Flags.MarkCompact
  | "generational" -> Flags.gc_strategy := Mo_config.Flags.Generational
  | "incremental" -> Flags.gc_strategy := Mo_config.Flags.Incremental
  | "enhancedOP" -> Flags.enhanced_orthogonal_persistence := true
  | "classicOP" -> Flags.enhanced_orthogonal_persistence := false
  | _ -> raise (Invalid_argument "gc_flags: Unexpected flag")
