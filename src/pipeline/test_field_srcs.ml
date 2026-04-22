(** Maintenance note:
    Update of the expected values could be done via [dune runtest --auto-promote].
*)

open Source

module Sexpr_set = Set.Make (struct
  type t = Wasm.Sexpr.sexpr

  let compare =
    let open Wasm.Sexpr in
    let rec compare_sexprs a b = List.compare compare_sexpr a b
    and compare_sexpr a b =
      match a, b with
      | Atom n1, Atom n2 -> String.compare n1 n2
      | Node (n1, as1), Node (n2, as2) ->
        (match String.compare n1 n2 with
        | 0 -> compare_sexprs as1 as2
        | o -> o)
      | Atom _, Node _ -> -1
      | Node _, Atom _ -> +1
    in
    compare_sexpr
end)

let rec remove_srcs =
  let open Wasm.Sexpr in
  let is_not_src = function
    | Atom _ -> true
    | Node (name, _) -> name <> "@@"
  in
  function
  | Atom _ as atom -> atom
  | Node (name, args) ->
    Node (name, List.(filter is_not_src args |> map remove_srcs))

let gather_let_srcs sexpr =
  let open Wasm.Sexpr in
  let srcs_of_labs name args =
    match name, args with
    | "Obj", Atom "Object" :: fields ->
      List.find_map
        (function
          | Node (lab, _typ :: _depr :: _region :: srcs) -> Some (lab, srcs)
          | _ -> None)
        fields
    | _, _ -> None
  in
  let rec go acc = function
    | Atom _ -> acc
    | Node (name, args) ->
      List.fold_left
        (fun acc arg ->
          let acc =
            match srcs_of_labs name args with
            | None -> acc
            | Some (lab, srcs) -> Sexpr_set.add (Node (lab, srcs)) acc
          in
          go acc arg)
        acc
        args
  in
  Sexpr_set.(go empty sexpr |> to_seq) |> List.of_seq

let arrange filename srcs_tbl : (module Mo_def.Arrange.S) =
  (module
    (Mo_def.Arrange.Make (struct
      let include_sources = false
      let include_type_rep = Mo_def.Arrange.With_type_rep (Some srcs_tbl)
      let include_types = true
      let include_parenthetical = false
      let include_docs = None
      let main_file = Some filename
    end)))

let show_msgs msgs = String.concat "\n" (List.map Diag.string_of_message msgs)

let show (result : (Mo_def.Syntax.prog * Mo_types.Field_sources.srcs_map) Diag.result) : unit =
  match result with
  | Error msgs -> Format.printf "Diagnostics:\n%s\n" (show_msgs msgs)
  | Ok ((prog, srcs), msgs) ->
    let filename = prog.note.Mo_def.Syntax.filename in
    let module Arrange = (val arrange filename srcs) in
    Format.printf "Ok:\n";
    Format.printf "Collected sources:\n";
    List.iter
      (fun srcs -> Format.printf "%s" (Wasm.Sexpr.to_string 80 srcs))
      (Arrange.prog prog |> gather_let_srcs);
    Format.printf "Sources table:\n";
    Seq.iter
      (fun (define, origins) ->
        Format.printf "%s:" (string_of_region define);
        Seq.iter
          (fun origin -> Format.printf " %s" (string_of_region origin))
          (Region_set.to_seq origins);
        Format.printf "\n")
      (Mo_types.Field_sources.Srcs_map.to_seq srcs);
    match msgs with
    | [] -> ()
    | _ :: _ -> Format.printf "With diagnostics:\n%s\n" (show_msgs msgs)

let run_get_sources_test source =
  let open Diag.Syntax in
  let infer_prog prog senv async_cap : Mo_types.Field_sources.srcs_map Diag.result =
    let filename = prog.Source.note.Mo_def.Syntax.filename in
    let* _typ, sscope =
      Mo_types.Cons.session ~scope:filename (fun () ->
        Mo_frontend.Typing.infer_prog
          senv
          None
          async_cap
          prog)
    in
    Diag.return sscope.fld_src_env
  in
  let filename = "test-field-srcs.mo" in
  Fun.protect
    (fun () ->
      Mo_config.Flags.typechecker_combine_srcs := true;
      show begin
        let* prog, _name = Pipeline.parse_string filename source in
        let async_cap = Mo_types.Async_cap.initial_cap () in
        let* srcs = infer_prog prog Pipeline.initial_stat_env async_cap in
        Diag.return (prog, srcs)
      end)
    ~finally:(fun () -> Mo_config.Flags.typechecker_combine_srcs := false)

let%expect_test "" =
  let s = {|persistent actor {
  class Class1() = self {
    public func meth(_ : Int) : Nat {
      return 1
    }
  };

  class Class2() = self {
    public func meth(_ : Nat) : Int {
      return 2
    }
  };

  public func test() : async Int {
    let c1 : Class2 = Class1();
    return c1.meth(42)
  }
}
|}
  in
  run_get_sources_test s;
  [%expect {|
    Ok:
    Collected sources:
    Sources table:
    test-field-srcs.mo:2.9-2.15: test-field-srcs.mo:2.9-2.15
    test-field-srcs.mo:3.17-3.21: test-field-srcs.mo:3.17-3.21
    test-field-srcs.mo:8.9-8.15: test-field-srcs.mo:8.9-8.15
    test-field-srcs.mo:9.17-9.21: test-field-srcs.mo:3.17-3.21 test-field-srcs.mo:9.17-9.21
    test-field-srcs.mo:14.15-14.19: test-field-srcs.mo:14.15-14.19 |}]

let run_compare_typed_asts_test filename =
  let open Diag.Syntax in
  let load_prog () =
    let* _libs, progs, _sscope, _cache =
      Mo_types.Cons.session ~scope:filename (fun () ->
        Pipeline.load_progs_cached
          ~check_actors:false
          Pipeline.parse_file
          [filename]
          Pipeline.initial_stat_env
          Mo_types.Type.Env.empty)
    in
    let prog, _deps, sscope = List.hd progs in
    Diag.return (prog, sscope.fld_src_env)
  in
  (* Ensure turning sources on will not change the AST. *)
  let* prog_no_combine, srcs_no_combine = load_prog () in
  let* prog_combine, srcs_combine =
    Fun.protect
      (fun () ->
        Mo_config.Flags.typechecker_combine_srcs := true;
        load_prog ())
      ~finally:(fun () -> Mo_config.Flags.typechecker_combine_srcs := false)
  in
  let module Arrange_no_combine = (val arrange filename srcs_no_combine) in
  let module Arrange_combine = (val arrange filename srcs_combine) in
  (* AST should match (modulo the field sources). *)
  let sexpr_prog_no_combine = Arrange_no_combine.prog prog_no_combine |> remove_srcs in
  let sexpr_prog_combine = Arrange_combine.prog prog_combine |> remove_srcs in
  if sexpr_prog_no_combine <> sexpr_prog_combine then
    failwith
      (Format.sprintf
        "Testing %s failed with an AST mismatch:\nAST 1:\n%sAST 2:\n%s\n%!"
        filename
        (Wasm.Sexpr.to_string 80 sexpr_prog_no_combine)
        (Wasm.Sexpr.to_string 80 sexpr_prog_combine));
  Diag.return ()

let get_mo_files_from_dir dir =
  let files = Sys.readdir dir in
  List.filter_map
    (fun file ->
      if Filename.extension file = ".mo"
      then Some (Filename.concat dir file)
      else None)
    (Array.to_list files)

let run_compare_typed_asts_tests_on_dir dir =
  let run_files = get_mo_files_from_dir dir in
  List.iter (fun file -> run_compare_typed_asts_test file |> ignore) run_files

let%test_unit "ASTs in test match with and without combining sources" =
  List.iter
    run_compare_typed_asts_tests_on_dir
    ["run"; "run-drun"; "perf"; "bench"]
