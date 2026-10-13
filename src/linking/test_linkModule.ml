open Wasm.Source
open Wasm_exts.Ast
open Wasm_exts.Types
open Wasm_exts.CustomModule

module Link = Linking.LinkModule

let phrase value = value @@ no_region
let name = Lib.Utf8.decode
let i64 value = phrase (Const (phrase (Wasm_exts.Values.I64 value)))
let i32 value = phrase (Const (phrase (Wasm_exts.Values.I32 value)))
let get_global index = phrase (GlobalGet (phrase index))
let call index = phrase (Call (phrase index))

let import module_name item_name descriptor = phrase {
  module_name = name module_name;
  item_name = name item_name;
  idesc = phrase descriptor;
}

let export export_name descriptor = phrase {
  name = name export_name;
  edesc = phrase descriptor;
}

let global mutability value = phrase {
  gtype = GlobalType (I64Type, mutability);
  value = phrase [i64 value];
}

let func body = phrase {ftype = phrase 0l; locals = []; body}

let extend module_ = {
  module_;
  name = empty_name_section;
  dylink0 = [];
  motoko = empty_motoko_sections;
  candid = empty_candid_sections;
  enhanced_orthogonal_persistence = None;
  source_mapping_url = None;
  wasm_features = [];
  target_features = [];
}

let base = extend {empty_module with
  types = [phrase (FuncType ([], []))];
  imports = [
    import "compute.a" "a_entry" (FuncImport (phrase 0l));
    import "compute.b" "b_entry" (FuncImport (phrase 0l));
  ];
  tables = [phrase {
    ttype = HugeTableType ({min = 1L; max = Some 1L}, FuncRefType)
  }];
  memories = [phrase {
    mtype = MemoryType ({min = 2L; max = None}, I64IndexType)
  }];
  globals = [global Immutable 65536L; global Mutable 65528L];
  funcs = [func [
    i64 60000L; i32 0l; i32 4l; phrase (MemoryInit (phrase 0l));
    call 0l; call 1l;
  ]];
  start = Some (phrase 2l);
  exports = [
    export "__heap_base" (GlobalExport (phrase 0l));
    export "__stack_pointer" (GlobalExport (phrase 1l));
    export "_start" (FuncExport (phrase 2l));
  ];
  datas = [phrase {dinit = "base"; dmode = phrase Passive}];
}

let library ?marker suffix contents =
  let mark = match marker with
    | None -> []
    | Some address -> [i64 address; i64 (Int64.of_int (Char.code contents.[0]));
        phrase (Store {ty = I64Type; align = 0; offset = 0L; sz = Some Pack8})]
  in
  let m = extend {empty_module with
    types = [phrase (FuncType ([], []))];
    imports = [
      import "env" "memory"
        (MemoryImport (MemoryType ({min = 1L; max = None}, I64IndexType)));
      import "env" "__indirect_function_table"
        (TableImport (HugeTableType ({min = 1L; max = None}, FuncRefType)));
      import "env" "__stack_pointer" (GlobalImport (GlobalType (I64Type, Mutable)));
      import "env" "__memory_base" (GlobalImport (GlobalType (I64Type, Immutable)));
      import "env" "__table_base" (GlobalImport (GlobalType (I64Type, Immutable)));
      import "GOT.func" (suffix ^ "_entry") (GlobalImport (GlobalType (I64Type, Mutable)));
      import "GOT.mem" (suffix ^ "_data") (GlobalImport (GlobalType (I64Type, Mutable)));
    ];
    globals = [global Immutable 0L];
    funcs = [func (mark @ [
      get_global 0l; phrase Drop;
      get_global 1l;
      phrase (Load {ty = I64Type; align = 0; offset = 0L; sz = Some (Pack8, ZX)});
      i64 (Int64.of_int (Char.code contents.[0]));
      phrase (Compare (Wasm_exts.Values.I64 I64Op.Ne));
      phrase (If (ValBlockType None, [phrase Unreachable], []));
      get_global 3l; get_global 2l; i64 1L;
      phrase (Binary (Wasm_exts.Values.I64 I64Op.Add));
      phrase (Compare (Wasm_exts.Values.I64 I64Op.Ne));
      phrase (If (ValBlockType None, [phrase Unreachable], []));
      get_global 4l; get_global 1l;
      phrase (Compare (Wasm_exts.Values.I64 I64Op.Ne));
      phrase (If (ValBlockType None, [phrase Unreachable], []));
    ])];
    exports = [
      export (suffix ^ "_entry") (FuncExport (phrase 0l));
      export (suffix ^ "_data") (GlobalExport (phrase 5l));
    ];
    elems = [phrase {
      index = phrase 0l;
      offset = phrase [get_global 2l];
      init = [phrase 0l];
    }];
    datas = [phrase {
      dinit = contents;
      dmode = phrase (Active {index = phrase 0l; offset = phrase [get_global 1l]});
    }];
  } in
  {m with dylink0 = [MemInfo {
    memory_size = Int32.of_int (String.length contents);
    memory_alignment = 3l;
    table_size = 1l;
    table_alignment = 0l;
  }]}

let exported (em : extended_module) export_name =
  List.find (fun (export : export) -> export.it.name = name export_name) em.module_.exports

let expect_error f =
  match f () with
  | () -> failwith "Expected LinkError"
  | exception Link.LinkError _ -> ()

let with_module (em : extended_module) f = {em with module_ = f em.module_}

let binary_round_trip (em : extended_module) =
  let unsigned n =
    let bytes = Buffer.create 8 in
    let rec emit n =
      let byte = Int32.to_int (Int32.logand n 127l) in
      let rest = Int32.shift_right_logical n 7 in
      Buffer.add_char bytes (Char.chr (if rest = 0l then byte else byte lor 128));
      if rest <> 0l then emit rest
    in
    emit n;
    Buffer.contents bytes
  in
  let _, encoded = Wasm_exts.CustomModuleEncode.encode {em with dylink0 = []} in
  let bytes = match em.dylink0 with
    | [] -> encoded
    | [MemInfo info] ->
      let fields = String.concat "" (List.map unsigned
        [info.memory_size; info.memory_alignment; info.table_size; info.table_alignment]) in
      let section = "\008dylink.0\001" ^ unsigned (Int32.of_int (String.length fields)) ^ fields in
      String.sub encoded 0 8 ^ "\000" ^
        unsigned (Int32.of_int (String.length section)) ^ section ^
        String.sub encoded 8 (String.length encoded - 8)
    | _ -> failwith "Unexpected test dylink metadata"
  in
  Wasm_exts.CustomModuleDecode.decode "binary-plugin-test" bytes

let test_table_decoding () =
  List.iter (fun table_type ->
    let em = extend {empty_module with tables = [phrase {ttype = table_type}]} in
    let decoded = binary_round_trip em in
    assert (List.map (fun table -> table.it.ttype) decoded.module_.tables = [table_type])
  ) [
    TableType ({min = Int32.min_int; max = Some Int32.minus_one}, FuncRefType);
    HugeTableType ({min = 4294967296L; max = Some 4294967297L}, FuncRefType);
  ];
  (* A zero encoded as six LEB bytes is invalid for a table32 limit even
     though the numeric value fits; use the width-specific decoder. *)
  let overlong32 = "\000asm\001\000\000\000\004\009\001\112\000\128\128\128\128\128\000" in
  match Wasm_exts.CustomModuleDecode.decode "overlong-table32" overlong32 with
  | _ -> failwith "Accepted overlong u32 table limit"
  | exception Wasm_exts.CustomModuleDecode.Code _ -> ()

let test_dylink_decoding () =
  let decode subsections =
    let section = "\008dylink.0" ^ subsections in
    let bytes = "\000asm\001\000\000\000\000" ^
      String.make 1 (Char.chr (String.length section)) ^ section in
    Wasm_exts.CustomModuleDecode.decode "dylink-length-test" bytes
  in
  ignore (decode "\001\004\000\000\000\000\002\001\000");
  List.iter (fun subsections ->
    match decode subsections with
    | _ -> failwith "Accepted malformed dylink subsection length"
    | exception Wasm_exts.CustomModuleDecode.Code _ -> ()
  ) [
    "\001\003\000\000\000\000"; (* Short memory-info subsection. *)
    "\001\005\000\000\000\000\000"; (* Surplus memory-info byte. *)
    "\001\005\000\000\000\000"; (* Subsection exceeds custom section. *)
    "\001\255\255\255\255\015"; (* Unsigned length exceeds input. *)
    "\002\000\000"; (* Short needed subsection. *)
    "\002\002\000\000"; (* Surplus needed byte. *)
    "\002\002\000"; (* Needed subsection exceeds custom section. *)
    "\001"; (* Missing subsection length. *)
  ]

let test_namespaced_libraries () =
  let a = binary_round_trip (library ~marker:60004L "" "aaa") in
  let b = binary_round_trip (library ~marker:60004L "" "bbbbb") in
  Link.validate_compute_plugin a;
  Link.validate_compute_plugin b;
  let expect_marker value = [
    i64 60004L;
    phrase (Load {ty = I64Type; align = 0; offset = 0L; sz = Some (Pack8, ZX)});
    i64 value; phrase (Compare (Wasm_exts.Values.I64 I64Op.Ne));
    phrase (If (ValBlockType None, [phrase Unreachable], []));
  ] in
  let main = with_module base (fun m -> {m with
    imports = List.map (fun import ->
      {import with it = {import.it with item_name = name "_entry"}}
    ) m.imports;
    funcs = [func (
      [call 0l] @ expect_marker 97L @
      [call 1l] @ expect_marker 98L @
      [i64 1L; phrase (CallIndirect (phrase 0l, phrase 0l))] @ expect_marker 97L @
      [i64 3L; phrase (CallIndirect (phrase 0l, phrase 0l))] @ expect_marker 98L
    )];
    exports = export "retained_rts_helper" (FuncExport (phrase 2l)) :: m.exports;
  }) in
  let one = Link.link ~finalize:false ~keep_exports:false main "compute.a" a in
  ignore (exported one "__heap_base");
  ignore (exported one "retained_rts_helper");
  assert (List.length one.module_.exports = List.length main.module_.exports);
  let two = Link.link ~finalize:false ~keep_exports:false one "compute.b" b in
  ignore (exported two "retained_rts_helper");
  assert (two.module_.imports = []);
  let _, bytes = Wasm_exts.CustomModuleEncode.encode (Link.finalize two) in
  ignore (Wasm_exts.CustomModuleDecode.decode "namespaced-plugin-test" bytes);
  if Array.length Sys.argv = 2 then begin
    let out = open_out_bin (Sys.argv.(1) ^ ".namespaced.wasm") in
    Fun.protect ~finally:(fun () -> close_out out) (fun () -> output_string out bytes)
  end

let test_linking () =
  let a = library "a" "aaa" in
  let b = library "b" "bbbbb" in
  Link.validate_compute_plugin a;
  Link.validate_compute_plugin b;
  let one = Link.link ~finalize:false base "compute.a" a in
  ignore (exported one "__heap_base");
  let two = Link.link ~finalize:false one "compute.b" b in
  assert (two.module_.imports = []);
  assert (List.map (fun segment -> segment.it.dinit) two.module_.datas =
    ["base"; "aaa"; "bbbbb"]);
  assert (List.for_all (fun segment -> segment.it.dmode.it = Passive) two.module_.datas);
  let start = match two.module_.start with
    | Some index -> List.nth two.module_.funcs (Int32.to_int index.it)
    | None -> failwith "Missing initialization function"
  in
  let loads = List.filter_map (fun instruction ->
    match instruction.it with MemoryInit index -> Some index.it | _ -> None
  ) start.it.body in
  assert (loads = [2l; 1l; 0l]);
  let addresses = List.filter_map (fun instruction ->
    match instruction.it with
    | Const {it = Wasm_exts.Values.I64 value; _} -> Some value
    | _ -> None
  ) start.it.body in
  assert (addresses = [65792L; 65536L; 60000L]);
  let heap = match (exported two "__heap_base").it.edesc.it with
    | GlobalExport index -> List.nth two.module_.globals (Int32.to_int index.it)
    | _ -> failwith "Heap export is not a global"
  in
  assert (heap.it.value.it = [i64 66048L]);
  assert (List.map (fun table -> table.it.ttype) two.module_.tables =
    [HugeTableType ({min = 5L; max = Some 5L}, FuncRefType)]);
  assert (List.map (fun (element : table_segment) -> element.it.offset.it) two.module_.elems =
    [[i64 1L]; [i64 2L]; [i64 3L]; [i64 4L]]);
  assert (List.map (fun (element : table_segment) -> List.map (fun index -> index.it) element.it.init)
    two.module_.elems = [[1l]; [1l]; [2l]; [2l]]);
  let final = Link.finalize two in
  assert (List.map (fun (export : export) -> Lib.Utf8.encode export.it.name) final.module_.exports =
    ["_start"]);
  let _, bytes = Wasm_exts.CustomModuleEncode.encode final in
  ignore (Wasm_exts.CustomModuleDecode.decode "linked-plugin-test" bytes);
  if Array.length Sys.argv = 2 then begin
    let out = open_out_bin Sys.argv.(1) in
    Fun.protect ~finally:(fun () -> close_out out) (fun () -> output_string out bytes)
  end;
  (* A later library must not silently shadow exports retained from an earlier
     library; its own references would otherwise resolve inconsistently. *)
  let duplicate = with_module b (fun m -> {m with
    exports = export "a_entry" (FuncExport (phrase 0l)) :: m.exports
  }) in
  expect_error (fun () -> ignore (Link.link ~finalize:false one "compute.b" duplicate))

let test_validation () =
  let good = library "a" "aaa" in
  let with_sizes memory_size table_size = {good with dylink0 = [MemInfo {
    memory_size; memory_alignment = 0l;
    table_size; table_alignment = 0l;
  }]} in
  let reject_module f = expect_error (fun () ->
    Link.validate_compute_plugin (with_module good f)
  ) in
  expect_error (fun () -> Link.validate_compute_plugin {good with dylink0 = []});
  expect_error (fun () -> Link.validate_compute_plugin {
    good with dylink0 = Needed ["libother.so"] :: good.dylink0
  });
  expect_error (fun () -> Link.validate_compute_plugin (with_sizes 524289l 1l));
  expect_error (fun () -> Link.validate_compute_plugin (with_sizes 3l 1048577l));
  expect_error (fun () -> ignore (
    Link.link ~finalize:false base "compute.a" (with_sizes Int32.max_int 1l)
  ));
  expect_error (fun () -> ignore (
    Link.link ~finalize:false base "compute.a" (with_sizes 3l Int32.max_int)
  ));
  reject_module (fun m -> {m with start = Some (phrase 0l)});
  reject_module (fun m -> {m with memories = base.module_.memories});
  reject_module (fun m -> {m with globals = [global Mutable 0L]});
  (* GOT.mem is relocated by adding its exported relative offset to the
     library base. A different type or an absolute address is incompatible
     with that transformation even when the input Wasm validates. *)
  List.iter (fun target ->
    let candidate = with_module good (fun m -> {m with globals = [target]}) in
    expect_error (fun () ->
      Link.validate_compute_plugin (binary_round_trip candidate)
    )
  ) [
    phrase {gtype = GlobalType (I32Type, Immutable); value = phrase [i32 0l]};
    phrase {gtype = GlobalType (I64Type, Immutable); value = phrase [get_global 1l]};
    global Immutable (-1L);
    global Immutable 4L;
  ];
  reject_module (fun m -> {m with exports = [
    export "a_entry" (FuncExport (phrase 0l));
    export "a_data" (GlobalExport (phrase 1l));
  ]});
  Link.validate_compute_plugin (binary_round_trip (with_module good (fun m ->
    {m with globals = [global Immutable 3L]}
  )));
  reject_module (fun m -> {m with imports = List.map (fun import ->
    match import.it.idesc.it with
    | MemoryImport _ -> {import with it = {import.it with
      idesc = phrase (MemoryImport (MemoryType ({min = 1L; max = None}, I32IndexType)))
    }}
    | _ -> import
  ) m.imports});
  reject_module (fun m -> {m with imports =
    import "ic0" "time" (FuncImport (phrase 0l)) :: m.imports
  });
  reject_module (fun m -> {m with imports =
    import "env" "compute_plugin_alloc" (FuncImport (phrase 0l)) :: m.imports
  });
  reject_module (fun m -> {m with funcs = [func [i64 0L; phrase (GlobalSet (phrase 3l))]]});
  reject_module (fun m -> {m with exports = List.hd m.exports :: m.exports});
  reject_module (fun m -> {m with datas = [phrase {dinit = "x"; dmode = phrase Passive}]});
  expect_error (fun () -> Link.validate_compute_plugin (with_sizes 2l 1l));
  expect_error (fun () -> Link.validate_compute_plugin (with_sizes 3l 0l));
  reject_module (fun m -> {m with exports = [export "bad" (FuncExport (phrase 999l))]});
  reject_module (fun m -> {m with exports = [export "bad" (GlobalExport (phrase 999l))]});
  (* PIC base imports are replaced by constants and removed during linking;
     their exports cannot be renamed to a surviving global index. *)
  List.iter (fun index ->
    let candidate = with_module good (fun m -> {m with exports =
      export "pic_alias" (GlobalExport (phrase index)) :: m.exports
    }) in
    expect_error (fun () ->
      Link.validate_compute_plugin (binary_round_trip candidate)
    )
  ) [1l; 2l];
  (* These reserved exports are called without arguments from a unit-returning
     start wrapper. Ordinary Wasm validation does not impose that signature. *)
  List.iter (fun name ->
    Link.validate_compute_plugin (binary_round_trip (with_module good (fun m ->
      {m with exports = export name (FuncExport (phrase 0l)) :: m.exports}
    )));
    reject_module (fun m -> {m with exports =
      export name (GlobalExport (phrase 5l)) :: m.exports
    });
    List.iter (fun (signature, body) ->
      let candidate = with_module good (fun m -> {m with
        types = m.types @ [phrase signature];
        funcs = m.funcs @ [phrase {ftype = phrase 1l; locals = []; body}];
        exports = export name (FuncExport (phrase 1l)) :: m.exports;
      }) in
      expect_error (fun () ->
        Link.validate_compute_plugin (binary_round_trip candidate)
      )
    ) [FuncType ([I64Type], []), []; FuncType ([], [I64Type]), [i64 0L]]
  ) ["__wasm_call_ctors"; "__wasm_apply_data_relocs"];
  reject_module (fun m -> {m with funcs = [phrase {ftype = phrase 999l; locals = []; body = []}]});
  reject_module (fun m -> {m with elems = [phrase {
    index = phrase 0l; offset = phrase [get_global 2l]; init = [phrase 999l]
  }]});
  reject_module (fun m -> {m with datas = [phrase {
    dinit = "x";
    dmode = phrase (Active {index = phrase 0l; offset = phrase [i64 0L]});
  }]});
  let offset = phrase [get_global 1l; i64 1L;
    phrase (Binary (Wasm_exts.Values.I64 I64Op.Add))] in
  Link.validate_compute_plugin (with_module good (fun m -> {m with datas = [phrase {
    dinit = "aa"; dmode = phrase (Active {index = phrase 0l; offset});
  }]}))

let () =
  test_linking ();
  test_table_decoding ();
  test_dylink_decoding ();
  test_namespaced_libraries ();
  test_validation ()
