module P = Codegen.Plugin_codegen
module C = Mo_values.Compute_plugin
module V = Mo_values.Value
module T = Mo_types.Type
module G = Codegen.InstrList

let a = C.Prepared.create ~owner:"prepared" ~name:"PreparedA"
let b = C.Prepared.create ~owner:"prepared" ~name:"PreparedB"

type operation = Seal | Read | Length

module type Operation = sig
  val descriptor : C.Prepared.t
  val operation : operation
end

module Generator (Op : Operation) = struct
  module Make (B : P.Backend) = struct
    let compile env specialization arguments =
      let value = G.concat arguments in
      let code = match Op.operation with
        | Seal -> B.Prepared.wrap Op.descriptor value
        | Read | Length ->
          let actual = List.hd specialization.C.argument_types in
          let bytes = B.Prepared.unwrap Op.descriptor actual value in
          (match Op.operation with
           | Length -> G.concat [bytes; B.Blob.len env;
               B.BigNum.from_word64 env]
           | Read ->
             let set_value, get_value = B.new_local env "prepared_value" in
             G.concat [bytes; set_value;
               get_value; B.Blob.payload_ptr_unskewed env;
               get_value; B.Blob.len env; B.Blob.of_ptr_size env]
           | Seal -> assert false)
      in P.{code; representation = Managed}
  end
end

let binding descriptor operation name = C.{
  id = name;
  name;
  arguments = Fixed (fun _ -> Ok [match operation with
    | Seal -> T.blob | Read | Length -> C.Prepared.typ descriptor]);
  type_parameters = [];
  result_type = (fun _ _ -> Ok (match operation with
    | Seal -> C.Prepared.typ descriptor | Read -> T.blob | Length -> T.nat));
  evaluate = pure (fun _ _ -> function
    | [value] -> Ok (match operation with
      | Seal -> C.Prepared.wrap descriptor (V.as_blob value)
      | Read -> V.Blob (C.Prepared.unwrap descriptor value)
      | Length -> V.Int (Mo_values.Numerics.Int.of_int
          (String.length (C.Prepared.unwrap descriptor value))))
    | _ -> Error "prepared fixture expects one argument");
}

let entry descriptor operation name =
  let module Op = struct let descriptor = descriptor let operation = operation end in
  binding descriptor operation name, (module Generator (Op) : P.Generator)

let () = P.register P.{
  abi = 1;
  name = "prepared";
  runtime = None;
  types = [a; b];
  imports = [];
  bindings = [
    entry a Seal "preparedA";
    entry b Seal "preparedB";
    entry a Read "preparedReadA";
    entry b Read "preparedReadB";
    entry a Length "preparedLenA";
  ];
}
