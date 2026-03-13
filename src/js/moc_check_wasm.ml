open Mo_config
open Source

module Js = Js_of_ocaml.Js

let position_of_pos pos =
  object%js
    val line = if pos.Source.line > 0 then pos.Source.line - 1 else 0
    val character = pos.Source.column
  end

let range_of_region at =
  object%js
    val start = position_of_pos at.Source.left
    val _end = position_of_pos at.Source.right
  end

let diagnostics_of_msg (msg : Diag.message) =
  Diag.(object%js
    val source = Js.string msg.at.Source.left.file
    val severity = if Diag.is_treated_as_error msg then 1 else 2
    val range = range_of_region msg.at
    val code = Js.string msg.code
    val category = Js.string msg.cat
    val message = Js.string msg.text
  end)

let diagnostics_of_msgs msgs =
  Array.of_list (List.map diagnostics_of_msg msgs)

let js_result result wrap_code =
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

(* Scope cache JS <-> OCaml conversion.
   The JS side treats scopes as opaque values (TypeScript type: unknown),
   always produced by the compiler and passed back unchanged.
   Hence the use of Obj.magic is legitimate here. *)

let scope_cache_from_js js_cache =
  let result = ref Mo_types.Type.Env.empty in
  let callback =
    Js.wrap_callback (fun v k _m ->
      let k = Js.to_string k in
      let v : Mo_frontend.Scope.t = Obj.magic v in
      result := Mo_types.Type.Env.add k v !result)
  in
  ignore (Js.Unsafe.meth_call js_cache "forEach" [|Js.Unsafe.inject callback|]);
  !result

let scope_cache_to_js cache =
  let js_map = Js.Unsafe.new_obj (Js.Unsafe.pure_js_expr "Map") [||] in
  Mo_types.Type.Env.iter
    (fun k v ->
       ignore (Js.Unsafe.meth_call js_map "set"
         [| Js.Unsafe.inject (Js.string k); Js.Unsafe.inject (Obj.magic v) |]))
    cache;
  js_map

(* Extra flags *)

let moc_args = Mo_args.inclusion_args
  @ Mo_args.warning_args
  @ Mo_args.error_args
  @ Mo_args.ai_args
  @ Mo_args.persistent_actors_args

let parse_extra_flags tokens =
  let argv = Array.append [|"moc-check"|] tokens in
  let current = ref 0 in
  Arg.parse_argv ~current argv moc_args (fun _ -> ()) "moc extra flags"

let js_fs = Js.Unsafe.pure_js_expr "require('fs')"

let () =
  Flags.compiled := true;
  let motoko = object%js
    val version = Js.string Source_id.id

    method check source =
      Mo_types.Cons.session (fun _ ->
        js_result
          (Stable_check.check_files ~enable_recovery:true [Js.to_string source])
          (fun _ -> Js.null))

    method checkWithScopeCache source js_scope_cache =
      let scope_cache = scope_cache_from_js js_scope_cache in
      let load_result =
        Mo_types.Cons.session (fun () ->
          Stable_check.load_progs_cached
            ~enable_type_recovery:true
            Stable_check.parse_file_with_recovery
            [Js.to_string source]
            Stable_check.initial_stat_env
            scope_cache)
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

    method stableCompatible pre post =
      js_result
        (Stable_check.stable_compatible (Js.to_string pre) (Js.to_string post))
        (fun _ -> Js.null)

    method candid source =
      Mo_types.Cons.session (fun _ ->
        js_result (Stable_check.generate_idl [Js.to_string source])
          (fun prog ->
            let open Idllib in
            let module WithComments = Arrange_idl.Make(struct let trivia = Some prog.note.Syntax.trivia end) in
            Js.some (Js.string (WithComments.string_of_prog prog))))

    method parseMotoko enable_recovery s =
      let main_file = "" in
      let parse_fn = if Js.Opt.get enable_recovery (fun () -> false)
        then Stable_check.parse_string_with_recovery
        else Stable_check.parse_string
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

    method parseMotokoWithDeps enable_recovery path s =
      let main_file = Js.to_string path in
      let s = Js.to_string s in
      let parse_fn = if Js.Opt.get enable_recovery (fun () -> false)
        then Stable_check.parse_string_with_recovery
        else Stable_check.parse_string
      in
      let result =
        let open Diag.Syntax in
        let* prog, _ = parse_fn main_file s in
        let* deps =
          Stable_check.Resolve_import.resolve (Stable_check.resolve_flags None) prog main_file
        in
        Diag.return (prog, deps)
      in
      js_result result (fun (prog, deps) ->
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
              |> List.map (fun dep -> Js.string (Stable_check.resolved_import_name dep))
              |> Array.of_list
              |> Js.array
          end))

    method parseMotokoTypedWithScopeCache enable_recovery paths js_scope_cache =
      let paths = paths |> Js.to_array |> Array.to_list |> List.map Js.to_string in
      let scope_cache =
        Js.Opt.case js_scope_cache
          (fun () -> Mo_types.Type.Env.empty)
          scope_cache_from_js
      in
      let recovery_enabled = Js.Opt.get enable_recovery (fun () -> false) in
      let parse_fn = if recovery_enabled
        then Stable_check.parse_file_with_recovery
        else Stable_check.parse_file
      in
      let load_result =
        Mo_types.Cons.session (fun () ->
          Stable_check.load_progs_cached ~enable_type_recovery:recovery_enabled
            parse_fn paths Stable_check.initial_stat_env scope_cache)
      in
      match load_result with
      | Ok ((_libs, progs, senv, scope_cache), msgs) ->
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
            end) in
            ( Arrange.prog_js prog
            , immediate_imports |> List.map Js.string |> Array.of_list |> Js.array
            , senv )
          ) |> Array.of_list
        in
        let result = Ok ((progs, scope_cache_to_js scope_cache), msgs) in
        js_result result (fun (progs, scope_cache) ->
          let progs =
            progs |> Array.map (fun (ast, immediate_imports, senv) ->
              object%js
                val ast = ast
                val immediateImports = immediate_imports
                val scope = Js.Unsafe.inject senv
              end)
            |> Js.array
          in
          Js.some (Js.array [| Js.Unsafe.inject progs; Js.Unsafe.inject scope_cache |]))
      | Error msgs ->
        object%js
          val diagnostics = Js.array (diagnostics_of_msgs msgs)
          val code = Js.null
        end

    method parseMotokoTyped paths =
      let paths = paths |> Js.to_array |> Array.to_list |> List.map Js.to_string in
      let load_result =
        Mo_types.Cons.session (fun () ->
          Stable_check.load_progs_cached ~enable_type_recovery:false
            Stable_check.parse_file paths Stable_check.initial_stat_env Mo_types.Type.Env.empty)
      in
      match load_result with
      | Ok ((_libs, progs, _senv_acc, _scope_cache), msgs) ->
        let progs =
          progs |> List.map (fun (prog, _immediate_imports, sscope) ->
            let open Mo_def in
            let module Arrange = Astjs.Make (struct
              let include_sources = true
              let include_type_rep = Arrange.With_type_rep (Some sscope.Mo_frontend.Scope.fld_src_env)
              let include_types = true
              let include_docs = Some prog.note.Syntax.trivia
              let include_parenthetical = false
              let main_file = Some prog.at.left.file
            end) in
            ( Arrange.prog_js prog
            , sscope )
          ) |> Array.of_list
        in
        let result = Ok (progs, msgs) in
        js_result result (fun progs ->
          let progs =
            progs |> Array.map (fun (ast, senv) ->
              object%js
                val ast = ast
                val scope = Js.Unsafe.inject senv
              end)
            |> Js.array
          in
          Js.some (Js.Unsafe.inject progs))
      | Error msgs ->
        object%js
          val diagnostics = Js.array (diagnostics_of_msgs msgs)
          val code = Js.null
        end

    method parseCandid s =
      let result = Idllib.Pipeline.parse_string (Js.to_string s) in
      js_result result (fun (prog, _) ->
        let rec js_of_sexpr = function
          | Wasm.Sexpr.Node (head, inner) ->
            Js.Unsafe.coerce (object%js
              val name = Js.string head
              val args = inner |> List.map js_of_sexpr |> Array.of_list |> Js.array |> Js.some
            end)
          | Wasm.Sexpr.Atom s ->
            Js.Unsafe.coerce (Js.string s)
        in
        Js.some (js_of_sexpr (Idllib.Arrange_idl.prog prog)))

    method contextualDotSuggestions scope raw_exp =
      let open Mo_frontend in
      let scope = (Obj.magic scope : Scope.t) in
      let exp = (Obj.magic raw_exp : Mo_def.Syntax.exp) in
      let receiver_ty = exp.note.Mo_def.Syntax.note_typ in
      let libs = scope.Scope.lib_env in
      let suggestions = Typing.contextual_dot_suggestions libs receiver_ty in
      Js.array (Array.of_seq (Seq.map (fun suggestion ->
        let open Typing in
        object%js
          val moduleUri = Js.string suggestion.module_url
          val funcName = Js.string suggestion.func_name
          val funcType = Js.string (Mo_types.Type.string_of_typ suggestion.func_ty)
        end
      ) (List.to_seq suggestions)))

    method contextualDotModule raw_exp =
      let open Mo_frontend in
      let exp = (Obj.magic raw_exp : Mo_def.Syntax.exp) in
      match Typing.contextual_dot_module exp with
      | Some (module_name_or_uri, func_name) -> Js.Opt.return (object%js
          val moduleNameOrUri = Js.string module_name_or_uri
          val funcName = Js.string func_name
        end)
      | None -> Js.Opt.empty

    method saveFile filename content =
      let dir = Js.string (Filename.dirname (Js.to_string filename)) in
      ignore (Js.Unsafe.meth_call js_fs "mkdirSync"
        [| Js.Unsafe.inject dir;
           Js.Unsafe.inject (Js.Unsafe.obj [| "recursive", Js.Unsafe.inject Js._true |]) |]);
      ignore (Js.Unsafe.meth_call js_fs "writeFileSync"
        [| Js.Unsafe.inject filename; Js.Unsafe.inject content |])

    method removeFile filename =
      ignore (Js.Unsafe.meth_call js_fs "unlinkSync" [| Js.Unsafe.inject filename |])

    method renameFile oldpath newpath =
      ignore (Js.Unsafe.meth_call js_fs "renameSync"
        [| Js.Unsafe.inject oldpath; Js.Unsafe.inject newpath |])

    method readFile path =
      Js.Unsafe.meth_call js_fs "readFileSync" [| Js.Unsafe.inject path; Js.Unsafe.inject (Js.string "utf8") |]

    method readDir path =
      Js.Unsafe.meth_call js_fs "readdirSync" [| Js.Unsafe.inject path |]

    method addPackage package dir =
      Flags.package_urls := Flags.M.add (Js.to_string package) (Js.to_string dir) !Flags.package_urls
    method clearPackage () = Flags.package_urls := Flags.M.empty
    method setCandidPath path = Flags.actor_idl_path := Some (Js.to_string path)

    method setActorAliases entries =
      let entries = Array.map (fun kv ->
        let kv = Js.to_array kv in
        Js.to_string (Array.get kv 0), Js.to_string (Array.get kv 1)) (Js.to_array entries) in
      Flags.actor_aliases := Flags.M.of_seq (Array.to_seq entries)

    method setPublicMetadata entries =
      Flags.public_metadata_names := Array.to_list (Array.map Js.to_string (Js.to_array entries))

    method setExtraFlags flags =
      parse_extra_flags (flags |> Js.to_array |> Array.map Js.to_string)

    val compiler = object%js
      method setTypecheckerCombineSrcs v =
        Flags.typechecker_combine_srcs := v
      method setBlobImportPlaceholders v =
        Flags.blob_import_placeholders := v
    end
  end in
  Js.Unsafe.global##.Motoko := motoko
