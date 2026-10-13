open Ir_def
open Ir
open Mo_types
open Mo_values

let tag = function
  | Type.Nat -> 1L | Type.Int -> 2L
  | Type.Nat8 -> 3L | Type.Nat16 -> 4L
  | Type.Nat32 -> 5L | Type.Nat64 -> 6L
  | Type.Int8 -> 7L | Type.Int16 -> 8L
  | Type.Int32 -> 9L | Type.Int64 -> 10L
  | Type.Bool -> 11L | Type.Char -> 12L
  | Type.Text -> 13L | Type.Blob -> 14L | Type.Principal -> 15L
  | _ -> invalid_arg "unsupported scalar comparator"

let supported = function
  | Type.Nat | Type.Int | Type.Nat8 | Type.Nat16 | Type.Nat32
  | Type.Nat64 | Type.Int8 | Type.Int16 | Type.Int32 | Type.Int64
  | Type.Bool | Type.Char | Type.Text | Type.Blob | Type.Principal -> true
  | _ -> false

let variable name exp = exp.it = VarE (Const, name)

let unit_tag label exp = match exp.it with
  | PrimE (TagPrim actual, [{it = PrimE (TupPrim, []); _}]) ->
    actual = label
  | _ -> false

let relation typ op left right exp = match exp.it with
  | PrimE (RelPrim (actual_typ, actual_op), [a; b]) ->
    Type.eq typ actual_typ && op = actual_op && left a && right b
  | _ -> false

let ordering less equal exp = match exp.it with
  | IfE (lt, lo, {it = IfE (eq, same, hi); _}) ->
    less lt && equal eq && unit_tag "less" lo &&
    unit_tag "equal" same && unit_tag "greater" hi
  | _ -> false

let order_type typ = match Type.normalize typ with
  | Type.Variant fields ->
    List.map (fun field -> field.Type.lab) fields =
      ["equal"; "greater"; "less"] &&
    List.for_all (fun field -> Type.eq field.Type.typ Type.unit) fields
  | _ -> false

let recognize ~resolve_prim exp = match exp.it with
  | FuncE (_, Type.Local, Type.Returns, [], [a; b], [result], body)
    when order_type result ->
    (match Type.normalize a.note, Type.normalize b.note with
    | Type.Prim prim, Type.Prim other when prim = other && supported prim ->
      let x = variable a.it and y = variable b.it in
      let rel op = relation (Type.Prim prim) op x y in
      let ordinary = ordering (rel Operator.LtOp) (rel Operator.EqOp) body in
      let boolean = prim = Type.Bool && match body.it with
        | IfE (eq, same, {it = IfE (test, hi, lo); _}) ->
          rel Operator.EqOp eq && unit_tag "equal" same && x test &&
          unit_tag "greater" hi && unit_tag "less" lo
        | _ -> false
      in
      let byte_sequence = match prim, body.it with
        | (Type.Text | Type.Blob),
          BlockE ([{it = LetD ({it = VarP name; _}, call); _}], tail) ->
          let primitive = match call.it with
            | PrimE (CallPrim [], [callee;
                {it = PrimE (TupPrim, [left; right]); _}])
              when x left && y right -> resolve_prim callee
            | PrimE ((OtherPrim _ as operation), [left; right])
              when x left && y right -> Some operation
            | _ -> None
          in
          let expected = if prim = Type.Text then "text_compare"
            else "blob_compare" in
          let zero exp = match exp.it with
            | LitE (Int8Lit n) -> Numerics.Int_8.eq n Numerics.Int_8.zero
            | _ -> false
          in
          let rel op = relation (Type.Prim Type.Int8) op
            (variable name) zero in
          primitive = Some (OtherPrim expected) &&
          ordering (rel Operator.LtOp) (rel Operator.EqOp) tail
        | _ -> false
      in
      if ordinary || boolean || byte_sequence then Some prim else None
    | _ -> None)
  | _ -> None
