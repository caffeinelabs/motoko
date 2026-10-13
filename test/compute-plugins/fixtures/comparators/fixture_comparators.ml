module P = Codegen.Plugin_codegen
module C = Mo_values.Compute_plugin
module G = Codegen.InstrList
module T = Mo_types.Type
module V = Mo_values.Value
open Wasm_exts.Types
let ( ^^ ) = G.( ^^ )

type operation = Compare | Pair | Unit | Nested

let order = T.Variant (List.map (fun lab ->
  T.{lab; typ = unit; src = empty_src}) ["equal"; "greater"; "less"])
let comparator t = T.Func (T.Local, T.Returns, [], [t; t], [order])
let schema operation = function
  | [t] ->
    let result = match operation with
      | Compare | Nested -> T.int
      | Pair -> T.Tup [T.int; t]
      | Unit -> T.unit
    in
    Ok ([comparator t; t; t], result)
  | _ -> Error "comparator fixture requires one type argument"

module Generator (Op : sig val operation : operation end) = struct
  module Make (B : P.Backend) = struct
    let compile env specialization arguments =
      let t = List.hd specialization.C.type_arguments in
      let representation, results = match Op.operation with
        | Compare | Nested -> P.Managed, [I64Type]
        | Pair -> P.Tuple 2, [I64Type; I64Type]
        | Unit -> P.Tuple 0, []
      in
      let body env primitive arguments =
        let closure = List.nth arguments 0
        and left = List.nth arguments 1 and right = List.nth arguments 2 in
        let comparison = match primitive with
          | Some prim ->
            left ^^ right ^^
            B.Func.share_code B.Func.Always env
              ("fixture_scalar_" ^ T.string_of_prim prim)
              ["left", I64Type; "right", I64Type] [I64Type]
              (fun env args -> G.concat args ^^ B.Compare.scalar env prim)
          | None ->
            let set_tag, get_tag = B.new_local env "order_tag" in
            B.Call.local env (comparator t) closure [left; right] ^^
            B.Variant.get_variant_tag env ^^ set_tag ^^
            get_tag ^^
            B.compile_eq_const (B.Variant.hash_variant_label env "less") ^^
            B.E.if1 I64Type (B.compile_unboxed_const (-1L))
              (get_tag ^^
               B.compile_eq_const (B.Variant.hash_variant_label env "equal") ^^
               B.E.if1 I64Type B.compile_unboxed_zero B.compile_unboxed_one)
        in
        match Op.operation with
          | Compare | Nested -> comparison ^^ B.BigNum.from_signed_word64 env
          | Pair -> comparison ^^ B.BigNum.from_signed_word64 env ^^ left
          | Unit -> comparison ^^ G.i Wasm_exts.Ast.Drop
      in
      let code = B.Compare.dispatch env ~comparator_index:0 arguments results
        (fun env primitive getters ->
          match Op.operation, primitive with
          | Nested, None ->
            B.Compare.dispatch env ~comparator_index:0 getters results body
          | _ -> body env primitive getters)
      in
      P.{code; representation}
  end
end

let binding operation id name = C.{
  id; name;
  arguments = Fixed (fun types -> Result.map fst (schema operation types));
  type_parameters = [Opaque];
  result_type = (fun types _ -> Result.map snd (schema operation types));
  evaluate = (fun context types _arguments values k ->
    match types, values with
    | [t], [closure; left; right] ->
      context.C.call (comparator t) closure [left; right] (function
        | Error message -> k (Error message)
        | Ok (V.Variant (label, _)) ->
          let sign = match label with
            | "less" -> -1 | "equal" -> 0 | _ -> 1 in
          let value = V.Int (Mo_values.Numerics.Int.of_int sign) in
          k (Ok (match operation with
            | Compare | Nested -> value
            | Pair -> V.Tup [value; left]
            | Unit -> V.Tup []))
        | _ -> k (Error "invalid comparator result"))
    | _ -> k (Error "invalid comparator arguments"));
}

let () = P.register P.{
  abi = 1; name = "fixture_comparators"; runtime = None;
  types = []; imports = [];
  bindings = [
    binding Compare "fixture_auto_compare" "fixtureAutoCompare",
      (module Generator (struct let operation = Compare end));
    binding Pair "fixture_auto_compare_pair" "fixtureAutoComparePair",
      (module Generator (struct let operation = Pair end));
    binding Unit "fixture_auto_compare_unit" "fixtureAutoCompareUnit",
      (module Generator (struct let operation = Unit end));
    binding Nested "fixture_nested_compare" "fixtureNestedCompare",
      (module Generator (struct let operation = Nested end));
  ];
}
