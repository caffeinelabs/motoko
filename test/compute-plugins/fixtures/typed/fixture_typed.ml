module P = Codegen.Plugin_codegen
module G = Codegen.InstrList
module T = Mo_types.Type

(* A small typed intrinsic: one byte describing each argument's static type.
   The deliberate function rejection also tests interpreter/backend errors. *)
let shape types =
  if List.exists (fun typ -> T.is_func (T.promote typ)) types then
    Error "fixtureShape does not support function values"
  else
    Ok (String.init (List.length types) (fun i ->
      match T.promote (List.nth types i) with
      | T.Prim T.Nat -> 'n'
      | T.Prim T.Text -> 't'
      | T.Prim T.Bool -> 'b'
      | T.Prim T.Blob -> 'B'
      | T.Tup _ -> 'T'
      | T.Array _ -> 'a'
      | T.Opt _ -> 'o'
      | T.Obj _ -> 'r'
      | T.Variant _ -> 'v'
      | _ -> '?'))

module Generator = struct
  module Make (B : P.Backend) = struct
    let compile env specialization _arguments =
      let code = match shape specialization.Mo_values.Compute_plugin.argument_types with
      | Error message -> B.E.trap_with env message
      | Ok bytes ->
        let set_output, get_output = B.new_local env "fixture_output" in
        let byte i = B.compile_unboxed_const
          (Int64.of_int (Char.code bytes.[i])) in
        G.concat [
          B.alloc_blob env
            (B.compile_unboxed_const (Int64.of_int (String.length bytes)));
          set_output;
          G.concat (List.init (String.length bytes) (fun i -> G.concat [
            byte i;
            get_output; B.Blob.payload_ptr_unskewed env;
            B.compile_add_const (Int64.of_int i);
            B.call env "fixture_typed" "transform";
          ]));
          get_output;
        ]
      in P.{code; representation = Managed}
  end
end

let binding = Mo_values.Compute_plugin.{
  id = "fixture_shape";
  name = "fixtureShape";
  arguments = Shared;
  type_parameters = [];
  result_type = (fun _ _ -> Ok T.blob);
  evaluate = Mo_values.Compute_plugin.pure (fun _ types _values ->
    Result.map (fun bytes -> Mo_values.Value.Blob bytes) (shape types));
}

module V = Mo_values.Value
module N = Mo_values.Numerics

let result typ = T.Variant [
  T.{lab = "err"; typ = text; src = empty_src};
  T.{lab = "ok"; typ; src = empty_src};
]
let summary = T.obj T.Object ["value", T.text; "count", T.nat64]
let record = T.obj T.Object [
  "value", T.text;
  "count", T.nat64;
  "children", T.Array (T.Opt T.text);
  "pair", T.Tup [T.Prim T.Nat8; T.bool];
  "nested", T.Opt (T.Opt T.text);
]

let object_value fields =
  V.Obj (List.fold_left (fun env (key, value) -> V.Env.add key value env)
    V.Env.empty fields)

let count text = V.Nat64 (N.Nat64.of_int (String.length text))

module Decode = struct
  module Make (B : P.Backend) = struct
    let compile env specialization arguments =
      let set_input, get_input = B.new_local env "fixture_text" in
      let value = match specialization.Mo_values.Compute_plugin.type_arguments with
        | [typ] when T.eq typ T.text -> get_input
        | [typ] when T.eq typ T.blob ->
          G.concat [get_input; B.Text.to_blob env]
        | [typ] when T.eq typ T.nat ->
          G.concat [get_input; B.Text.size env; B.BigNum.from_word64 env]
        | _ ->
          B.Object.lit_raw env [
            "value", (fun () -> get_input);
            "count", (fun () -> G.concat [get_input; B.Text.size env;
              B.BoxedWord64.box env T.Nat64]);
          ]
      in
      P.{representation = Managed;
        code = G.concat [G.concat arguments; set_input; B.Variant.inject env "ok" value]}
  end
end

let decode_binding = Mo_values.Compute_plugin.{
  id = "fixture_decode";
  name = "fixtureDecode";
  arguments = Fixed (fun _ -> Ok [T.text]);
  type_parameters = [Mo_values.Compute_plugin.Concrete];
  result_type = (fun type_arguments _ -> match type_arguments with
    | [typ] when List.exists (T.eq typ) [T.text; T.blob; T.nat; summary] ->
      Ok (result typ)
    | _ -> Error "fixtureDecode supports Text, Blob, Nat, or {value : Text; count : Nat64}");
  evaluate = Mo_values.Compute_plugin.pure (fun type_arguments _ values -> match type_arguments, values with
    | [typ], [V.Text text] ->
      let value =
        if T.eq typ T.text then V.Text text
        else if T.eq typ T.blob then V.Blob text
        else if T.eq typ T.nat then V.Int (N.Int.of_int (String.length text))
        else object_value ["value", V.Text text; "count", count text]
      in Ok (V.Variant ("ok", value))
    | _ -> Error "invalid fixtureDecode metadata");
}

module Record = struct
  module Make (B : P.Backend) = struct
    let compile env _specialization arguments =
      let set_input, get_input = B.new_local env "fixture_text" in
      let fields = [
        "value", (fun () -> get_input);
        "count", (fun () -> G.concat [get_input; B.Text.size env;
          B.BoxedWord64.box env T.Nat64]);
        "children", (fun () ->
          B.Arr.tabulate env (B.compile_unboxed_const 257L) (fun index ->
            G.concat [index; B.compile_eq_const 0L;
              B.E.if1 Wasm_exts.Types.I64Type (B.Opt.null_lit env)
                (B.Opt.inject env T.text get_input)]));
        "pair", (fun () -> G.concat [
          B.compile_unboxed_const 255L; B.TaggedSmallWord.msb_adjust T.Nat8;
          B.TaggedSmallWord.tag env T.Nat8; B.Bool.lit true;
          B.Tuple.from_stack env 2]);
        "nested", (fun () ->
          B.Opt.inject env (T.Opt T.text) (B.Opt.null_lit env));
      ] in
      P.{representation = Managed; code = G.concat [G.concat arguments; set_input;
        B.Variant.inject env "ok" (B.Object.lit_raw env fields)]}
  end
end

let record_binding = Mo_values.Compute_plugin.{
  id = "fixture_record";
  name = "fixtureRecord";
  arguments = Fixed (fun _ -> Ok [T.text]);
  type_parameters = [];
  result_type = (fun _ _ -> Ok (result record));
  evaluate = Mo_values.Compute_plugin.pure (fun _ _ -> function
    | [V.Text text] -> Ok (V.Variant ("ok", object_value [
        "value", V.Text text;
        "count", count text;
        "children", V.Array (Array.init 257 (function
          | 0 -> V.Null | _ -> V.Opt (V.Text text)));
        "pair", V.Tup [V.Nat8 (N.Nat8.of_int 255); V.Bool true];
        "nested", V.Opt V.Null;
      ]))
    | _ -> Error "invalid fixtureRecord metadata");
}

module EarlyError = struct
  module Make (B : P.Backend) = struct
    let compile env _specialization arguments =
      P.{representation = Managed; code = G.concat [G.concat arguments;
      B.Func.share_code1 B.Func.Always env "fixture_partial_array"
        ("text", Wasm_exts.Types.I64Type) [Wasm_exts.Types.I64Type]
        (fun env get_text ->
          B.Variant.inject env "ok"
            (B.Arr.tabulate env (B.compile_unboxed_const 1024L)
              (fun index -> G.concat [
                index; B.compile_eq_const 1L;
                B.E.if0 (G.concat [
                  B.Variant.inject env "err" (B.Text.lit env "early error");
                  G.i Wasm_exts.Ast.Return;
                ]) G.nop;
                get_text;
              ]))) ]}
  end
end

let early_error_binding = Mo_values.Compute_plugin.{
  id = "fixture_early_error";
  name = "fixtureEarlyError";
  arguments = Fixed (fun _ -> Ok [T.text]);
  type_parameters = [];
  result_type = (fun _ _ -> Ok (result (T.Array T.text)));
  evaluate = Mo_values.Compute_plugin.pure (fun _ _ _ -> Ok (V.Variant ("err", V.Text "early error")));
}

let () = P.register P.{
  abi = 1;
  name = "fixture_typed";
  runtime = Some "runtime.wasm";
  types = [];
  (* Deliberately shares the echo runtime's symbol with a different type. *)
  imports = ["transform", Wasm_exts.Types.[I64Type; I64Type], []];
  bindings = [
    binding, (module Generator);
    decode_binding, (module Decode);
    record_binding, (module Record);
    early_error_binding, (module EarlyError);
  ];
}
