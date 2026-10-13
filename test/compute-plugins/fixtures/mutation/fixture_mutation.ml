module P = Codegen.Plugin_codegen
module C = Mo_values.Compute_plugin
module G = Codegen.InstrList
module T = Mo_types.Type
module V = Mo_values.Value
module N = Mo_values.Numerics

let local ins outs = T.Func (T.Local, T.Returns, [], ins, outs)
let array typ = T.Array (T.Mut typ)
let cell typ = T.obj T.Object ["value", T.Mut typ]
let record typ = T.obj T.Object [
  "value", T.Mut typ; "spare", T.Mut typ; "marker", T.nat;
]

type operation = Get | Set | Tabulate | Record | Read | Write | Operands
  | Fields | FieldOperands | Marker | EarlyArray | EarlyRecord

let schema operation types = match operation, types with
  | Get, [typ] -> Ok ([array typ; T.nat], typ)
  | Set, [typ] -> Ok ([array typ; T.nat; typ], T.unit)
  | Tabulate, [typ] -> Ok ([T.nat; local [T.nat] [typ]], array typ)
  | Record, [typ] -> Ok ([typ], record typ)
  | Read, [typ] -> Ok ([cell typ], typ)
  | Write, [typ] -> Ok ([cell typ; typ], T.unit)
  | Operands, [typ] ->
    Ok ([local [] [array typ]; local [] [T.nat]; local [] [typ]], T.unit)
  | Fields, [typ] -> Ok ([local [] [typ]; local [] [typ]], record typ)
  | FieldOperands, [typ] ->
    Ok ([local [] [cell typ]; local [] [typ]], T.unit)
  | Marker, [] -> Ok ([T.obj T.Object ["marker", T.nat]], T.nat)
  | (EarlyArray | EarlyRecord), [] -> Ok ([], T.text)
  | _ -> Error "invalid mutation fixture type arguments"

module type Operation = sig val operation : operation end
module Generator (Op : Operation) = struct
  module Make (B : P.Backend) = struct
    let compile env specialization getters =
      let types = specialization.C.type_arguments in
      let record env typ first second = B.Object.lit env (record typ) [
        "value", typ, (fun () -> first);
        "spare", typ, (fun () -> second);
        "marker", T.nat, (fun () -> G.concat [
          B.compile_unboxed_const 7L; B.BigNum.from_word64 env]);
      ] in
      let code = match Op.operation, types, getters with
        | Get, [_], [arr; index] ->
          G.concat [arr; index; B.BigNum.to_word64 env; B.Arr.get env]
        | Set, [typ], [arr; index; value] -> G.concat [
          B.Arr.set env (array typ) typ arr
            (G.concat [index; B.BigNum.to_word64 env]) value;
          B.Tuple.compile_unit env;
        ]
        | Tabulate, [typ], [length; callback] ->
          B.Arr.tabulate_mut env
            (G.concat [length; B.BigNum.to_word64 env]) (fun index ->
              B.Call.local env (local [T.nat] [typ]) callback
                [G.concat [index; B.BigNum.from_word64 env]])
        | Record, [typ], [value] -> record env typ value value
        | Read, [typ], [object_] ->
          G.concat [object_; B.Object.load env (cell typ) "value"]
        | Write, [typ], [object_; value] -> G.concat [
          B.Object.set env (cell typ) "value" typ object_ value;
          B.Tuple.compile_unit env;
        ]
        | Operands, [typ], [arr; index; value] -> G.concat [
          B.Arr.set env (array typ) typ
            (B.Call.local env (local [] [array typ]) arr [])
            (G.concat [B.Call.local env (local [] [T.nat]) index [];
              B.BigNum.to_word64 env])
            (B.Call.local env (local [] [typ]) value []);
          B.Tuple.compile_unit env;
        ]
        | FieldOperands, [typ], [object_; value] -> G.concat [
          B.Object.set env (cell typ) "value" typ
            (B.Call.local env (local [] [cell typ]) object_ [])
            (B.Call.local env (local [] [typ]) value []);
          B.Tuple.compile_unit env;
        ]
        | Fields, [typ], [first; second] -> record env typ
            (B.Call.local env (local [] [typ]) first [])
            (B.Call.local env (local [] [typ]) second [])
        | Marker, [], [object_] -> G.concat [object_;
            B.Object.load env (T.obj T.Object ["marker", T.nat]) "marker"]
        | (EarlyArray | EarlyRecord), [], [] ->
          B.Func.share_code B.Func.Always env
            (if Op.operation = EarlyArray then "early_mut_array"
             else "early_mut_record") [] [Wasm_exts.Types.I64Type]
            (fun env _ ->
              let early = G.concat [B.Text.lit env "early";
                G.i Wasm_exts.Ast.Return] in
              G.concat [
                (if Op.operation = EarlyArray then
                  B.Arr.tabulate_mut env (B.compile_unboxed_const 1024L)
                    (fun index -> G.concat [index; B.compile_eq_const 2L;
                      B.E.if0 early G.nop; B.Text.lit env "payload"])
                 else record env T.text (B.Text.lit env "first") early);
                G.i Wasm_exts.Ast.Drop; B.Text.lit env "unreachable";
              ])
        | _ -> B.E.trap_with env "invalid mutation fixture arguments"
      in P.{code; representation = Managed}
  end
end

let object_value first second =
  V.Obj (List.fold_left (fun fields (label, value) ->
    V.Env.add label value fields) V.Env.empty [
      "value", V.Mut (ref first); "spare", V.Mut (ref second);
      "marker", V.Int (N.Int.of_int 7);
    ])

let index length = function
  | V.Int value when N.Int.ge value N.Int.zero &&
      N.Int.lt value (N.Int.of_int length) -> Ok (N.Int.to_int value)
  | _ -> Error "Array index out of bounds"

let set_array arr idx value = match index (Array.length arr) idx with
  | Error message -> Error message
  | Ok idx -> (match arr.(idx) with
      | V.Mut slot -> slot := value; Ok (V.Tup [])
      | _ -> Error "expected mutable array element")

let evaluator operation evaluator types _ values k =
  match operation, types, values with
  | Get, [_], [V.Array arr; idx] ->
    k (match index (Array.length arr) idx with
      | Error message -> Error message
      | Ok idx -> (match arr.(idx) with
          | V.Mut slot -> Ok !slot
          | _ -> Error "expected mutable array element"))
  | Set, [_], [V.Array arr; idx; value] -> k (set_array arr idx value)
  | Tabulate, [typ], [V.Int length; callback] ->
    let length = N.Int.to_int length in
    let arr = Array.init length (fun _ -> V.Mut (ref V.Null)) in
    let rec fill idx =
      if idx = length then k (Ok (V.Array arr)) else
      evaluator.C.call (local [T.nat] [typ]) callback
        [V.Int (N.Int.of_int idx)] (function
          | Error message -> k (Error message)
          | Ok value -> (match arr.(idx) with
              | V.Mut slot -> slot := value; fill (idx + 1)
              | _ -> k (Error "expected mutable array element")))
    in fill 0
  | Record, [_], [value] -> k (Ok (object_value value value))
  | Read, [_], [V.Obj fields] ->
    (match V.Env.find "value" fields with
     | V.Mut value -> k (Ok !value)
     | _ -> k (Error "expected mutable record field"))
  | Write, [_], [V.Obj fields; value] ->
    (match V.Env.find "value" fields with
     | V.Mut slot -> slot := value; k (Ok (V.Tup []))
     | _ -> k (Error "expected mutable record field"))
  | Operands, [typ], [arr; idx; value] ->
    evaluator.C.call (local [] [array typ]) arr [] (function
      | Error message -> k (Error message)
      | Ok (V.Array arr) ->
        evaluator.C.call (local [] [T.nat]) idx [] (function
          | Error message -> k (Error message)
          | Ok idx -> evaluator.C.call (local [] [typ]) value [] (function
              | Error message -> k (Error message)
              | Ok value -> k (set_array arr idx value)))
      | Ok _ -> k (Error "expected mutable array"))
  | FieldOperands, [typ], [object_; value] ->
    evaluator.C.call (local [] [cell typ]) object_ [] (function
      | Error message -> k (Error message)
      | Ok (V.Obj fields) ->
        evaluator.C.call (local [] [typ]) value [] (function
          | Error message -> k (Error message)
          | Ok value -> (match V.Env.find "value" fields with
              | V.Mut slot -> slot := value; k (Ok (V.Tup []))
              | _ -> k (Error "expected mutable record field")))
      | Ok _ -> k (Error "expected mutable record"))
  | Fields, [typ], [first; second] ->
    evaluator.C.call (local [] [typ]) first [] (function
      | Error message -> k (Error message)
      | Ok first -> evaluator.C.call (local [] [typ]) second [] (function
          | Error message -> k (Error message)
          | Ok second -> k (Ok (object_value first second))))
  | Marker, [], [V.Obj fields] -> k (Ok (V.Env.find "marker" fields))
  | (EarlyArray | EarlyRecord), [], [] -> k (Ok (V.Text "early"))
  | _ -> k (Error "invalid mutation fixture arguments")

let binding name operation parameters = C.{
  id = name;
  name;
  arguments = Fixed (fun types -> Result.map fst (schema operation types));
  type_parameters = List.init parameters (fun _ -> Opaque);
  result_type = (fun types _ -> Result.map snd (schema operation types));
  evaluate = evaluator operation;
}

let () = P.register P.{
  abi = 1; name = "fixture_mutation"; runtime = None;
  types = []; imports = [];
  bindings = [
    binding "fixtureMutGet" Get 1,
      (module Generator (struct let operation = Get end));
    binding "fixtureMutSet" Set 1,
      (module Generator (struct let operation = Set end));
    binding "fixtureMutTabulate" Tabulate 1,
      (module Generator (struct let operation = Tabulate end));
    binding "fixtureMutRecord" Record 1,
      (module Generator (struct let operation = Record end));
    binding "fixtureMutRead" Read 1,
      (module Generator (struct let operation = Read end));
    binding "fixtureMutWrite" Write 1,
      (module Generator (struct let operation = Write end));
    binding "fixtureMutOperands" Operands 1,
      (module Generator (struct let operation = Operands end));
    binding "fixtureMutFieldOperands" FieldOperands 1,
      (module Generator (struct let operation = FieldOperands end));
    binding "fixtureMutFields" Fields 1,
      (module Generator (struct let operation = Fields end));
    binding "fixtureMutMarker" Marker 0,
      (module Generator (struct let operation = Marker end));
    binding "fixtureMutEarlyArray" EarlyArray 0,
      (module Generator (struct let operation = EarlyArray end));
    binding "fixtureMutEarlyRecord" EarlyRecord 0,
      (module Generator (struct let operation = EarlyRecord end));
  ];
}
