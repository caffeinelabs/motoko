open Value
open Numerics

module T = Mo_types.Type

(* Operators *)

type unop =
  | PosOp                                       (* +x *)
  | NegOp                                       (* -x *)
  | NotOp                                       (* bitwise negation *)

type binop =
  | AddOp                                       (* x+y *)
  | SubOp                                       (* x-y *)
  | MulOp                                       (* x*y *)
  | DivOp                                       (* x/y *)
  | ModOp                                       (* x%y *)
  | PowOp                                       (* x^y *)
  | AndOp                                       (* bitwise operators... *)
  | OrOp
  | XorOp
  | ShLOp
  | ShROp
  | RotLOp
  | RotROp
  | WAddOp                                      (* wrapping operators... *)
  | WSubOp
  | WMulOp
  | WPowOp
  | CatOp                                       (* concatenation *)

type relop =
  | EqOp                                        (* x=y *)
  | NeqOp                                       (* x!=y *)
  | LtOp                                        (* x<y *)
  | GtOp                                        (* x>y *)
  | LeOp                                        (* x<=y *)
  | GeOp                                        (* x>=y *)

let impossible _ = raise (Invalid_argument "operator called for None")


(* Unary operators *)

(* bit-wise unops *)
let bit_unop (fnat8, fnat16, fnat32, fnat64, fint8, fint16, fint32, fint64) = function
  | T.Nat8 -> fun v -> Nat8 (fnat8 (as_nat8 v))
  | T.Nat16 -> fun v -> Nat16 (fnat16 (as_nat16 v))
  | T.Nat32 -> fun v -> Nat32 (fnat32 (as_nat32 v))
  | T.Nat64 -> fun v -> Nat64 (fnat64 (as_nat64 v))
  | T.Int8 -> fun v -> Int8 (fint8 (as_int8 v))
  | T.Int16 -> fun v -> Int16 (fint16 (as_int16 v))
  | T.Int32 -> fun v -> Int32 (fint32 (as_int32 v))
  | T.Int64 -> fun v -> Int64 (fint64 (as_int64 v))
  | _ -> raise (Invalid_argument "unop")

(* types that support sign operations (+, -)  *)
let sign_unop fint (fint8, fint16, fint32, fint64) ffloat = function
  | T.Int -> fun v -> Int (fint (as_int v))
  | T.Int8 -> fun v -> Int8 (fint8 (as_int8 v))
  | T.Int16 -> fun v -> Int16 (fint16 (as_int16 v))
  | T.Int32 -> fun v -> Int32 (fint32 (as_int32 v))
  | T.Int64 -> fun v -> Int64 (fint64 (as_int64 v))
  | T.Float -> fun v -> Float (ffloat (as_float v))
  | _ -> raise (Invalid_argument "unop")

let unop op t =
  match t with
  | T.Prim p ->
    (match op with
    | PosOp -> Fun.(sign_unop id (id, id, id, id) id p)
    | NegOp ->
      sign_unop
        Int.neg
        (Int_8.neg, Int_16.neg, Int_32.neg, Int_64.neg)
        Float.neg
        p
    | NotOp -> bit_unop
      (Nat8.not, Nat16.not, Nat32.not, Nat64.not,
       Int_8.not, Int_16.not, Int_32.not, Int_64.not)
      p
    )
  | T.Non -> impossible
  | _ -> raise (Invalid_argument "unop")


(* Binary operators *)

let text_binop ftext = function
  | T.Text -> fun v1 v2 -> Text (ftext (as_text v1) (as_text v2))
  | _ -> raise (Invalid_argument "binop")

let fixed_binop (fnat8, fnat16, fnat32, fnat64, fint8, fint16, fint32, fint64) = function
  | T.Nat8 -> fun v1 v2 -> Nat8 (fnat8 (as_nat8 v1) (as_nat8 v2))
  | T.Nat16 -> fun v1 v2 -> Nat16 (fnat16 (as_nat16 v1) (as_nat16 v2))
  | T.Nat32 -> fun v1 v2 -> Nat32 (fnat32 (as_nat32 v1) (as_nat32 v2))
  | T.Nat64 -> fun v1 v2 -> Nat64 (fnat64 (as_nat64 v1) (as_nat64 v2))
  | T.Int8 -> fun v1 v2 -> Int8 (fint8 (as_int8 v1) (as_int8 v2))
  | T.Int16 -> fun v1 v2 -> Int16 (fint16 (as_int16 v1) (as_int16 v2))
  | T.Int32 -> fun v1 v2 -> Int32 (fint32 (as_int32 v1) (as_int32 v2))
  | T.Int64 -> fun v1 v2 -> Int64 (fint64 (as_int64 v1) (as_int64 v2))
  | _ -> raise (Invalid_argument "binop")

let num_binop fnat fint ffixed ffloat = function
  | T.Nat -> fun v1 v2 -> Int (fnat (as_int v1) (as_int v2))
  | T.Int -> fun v1 v2 -> Int (fint (as_int v1) (as_int v2))
  | T.Float -> fun v1 v2 -> Float (ffloat (as_float v1) (as_float v2))
  | t -> fixed_binop ffixed t

let binop op t =
  match t with
  | T.Prim p ->
    (match op with
    | AddOp -> num_binop Nat.add Int.add (Nat8.add, Nat16.add, Nat32.add, Nat64.add, Int_8.add, Int_16.add, Int_32.add, Int_64.add) Float.add p
    | SubOp -> num_binop Nat.sub Int.sub (Nat8.sub, Nat16.sub, Nat32.sub, Nat64.sub, Int_8.sub, Int_16.sub, Int_32.sub, Int_64.sub) Float.sub p
    | MulOp -> num_binop Nat.mul Int.mul (Nat8.mul, Nat16.mul, Nat32.mul, Nat64.mul, Int_8.mul, Int_16.mul, Int_32.mul, Int_64.mul) Float.mul p
    | DivOp -> num_binop Nat.div Int.div (Nat8.div, Nat16.div, Nat32.div, Nat64.div, Int_8.div, Int_16.div, Int_32.div, Int_64.div) Float.div p
    | ModOp -> num_binop Nat.rem Int.rem (Nat8.rem, Nat16.rem, Nat32.rem, Nat64.rem, Int_8.rem, Int_16.rem, Int_32.rem, Int_64.rem) Float.rem p
    | PowOp -> num_binop Nat.pow Int.pow (Nat8.pow, Nat16.pow, Nat32.pow, Nat64.pow, Int_8.pow, Int_16.pow, Int_32.pow, Int_64.pow) Float.pow p
    | AndOp -> fixed_binop (Nat8.and_, Nat16.and_, Nat32.and_, Nat64.and_, Int_8.and_, Int_16.and_, Int_32.and_, Int_64.and_) p
    | OrOp  -> fixed_binop (Nat8.or_, Nat16.or_, Nat32.or_, Nat64.or_, Int_8.or_, Int_16.or_, Int_32.or_, Int_64.or_) p
    | XorOp -> fixed_binop (Nat8.xor, Nat16.xor, Nat32.xor, Nat64.xor, Int_8.xor, Int_16.xor, Int_32.xor, Int_64.xor) p
    | ShLOp -> fixed_binop (Nat8.shl, Nat16.shl, Nat32.shl, Nat64.shl, Int_8.shl, Int_16.shl, Int_32.shl, Int_64.shl) p
    | ShROp -> fixed_binop (Nat8.shr, Nat16.shr, Nat32.shr, Nat64.shr, Int_8.shr, Int_16.shr, Int_32.shr, Int_64.shr) p
    | RotLOp -> fixed_binop (Nat8.rotl, Nat16.rotl, Nat32.rotl, Nat64.rotl, Int_8.rotl, Int_16.rotl, Int_32.rotl, Int_64.rotl) p
    | RotROp -> fixed_binop (Nat8.rotr, Nat16.rotr, Nat32.rotr, Nat64.rotr, Int_8.rotr, Int_16.rotr, Int_32.rotr, Int_64.rotr) p
    | WAddOp -> fixed_binop (Nat8.wadd, Nat16.wadd, Nat32.wadd, Nat64.wadd, Int_8.wadd, Int_16.wadd, Int_32.wadd, Int_64.wadd) p
    | WSubOp -> fixed_binop (Nat8.wsub, Nat16.wsub, Nat32.wsub, Nat64.wsub, Int_8.wsub, Int_16.wsub, Int_32.wsub, Int_64.wsub) p
    | WMulOp -> fixed_binop (Nat8.wmul, Nat16.wmul, Nat32.wmul, Nat64.wmul, Int_8.wmul, Int_16.wmul, Int_32.wmul, Int_64.wmul) p
    | WPowOp -> fixed_binop (Nat8.wpow, Nat16.wpow, Nat32.wpow, Nat64.wpow, Int_8.wpow, Int_16.wpow, Int_32.wpow, Int_64.wpow) p
    | CatOp -> text_binop (^) p
    )
  | T.Non -> impossible
  | _ -> raise (Invalid_argument "binop")


(* Relational operators *)

let num_relop fnat fint (fnat8, fnat16, fnat32, fnat64, fint8, fint16, fint32, fint64) ffloat = function
  | T.Nat -> fun v1 v2 -> Bool (fnat (as_int v1) (as_int v2))
  | T.Nat8 -> fun v1 v2 -> Bool (fnat8 (as_nat8 v1) (as_nat8 v2))
  | T.Nat16 -> fun v1 v2 -> Bool (fnat16 (as_nat16 v1) (as_nat16 v2))
  | T.Nat32 -> fun v1 v2 -> Bool (fnat32 (as_nat32 v1) (as_nat32 v2))
  | T.Nat64 -> fun v1 v2 -> Bool (fnat64 (as_nat64 v1) (as_nat64 v2))
  | T.Int -> fun v1 v2 -> Bool (fint (as_int v1) (as_int v2))
  | T.Int8 -> fun v1 v2 -> Bool (fint8 (as_int8 v1) (as_int8 v2))
  | T.Int16 -> fun v1 v2 -> Bool (fint16 (as_int16 v1) (as_int16 v2))
  | T.Int32 -> fun v1 v2 -> Bool (fint32 (as_int32 v1) (as_int32 v2))
  | T.Int64 -> fun v1 v2 -> Bool (fint64 (as_int64 v1) (as_int64 v2))
  | T.Float -> fun v1 v2 -> Bool (ffloat (as_float v1) (as_float v2))
  | _ -> raise (Invalid_argument "relop")

let ord_relop fnat fint fwords ffloat fchar ftext fblob = function
  | T.Char -> fun v1 v2 -> Bool (fchar (as_char v1) (as_char v2))
  | T.Text -> fun v1 v2 -> Bool (ftext (as_text v1) (as_text v2))
  | T.Blob | T.Principal -> fun v1 v2 -> Bool (ftext (as_blob v1) (as_blob v2))
  | t -> num_relop fnat fint fwords ffloat t

let eq_relop fnat fint fwords ffloat fchar ftext fblob fnull fbool = function
  | T.Null -> fun v1 v2 -> Bool (fnull (as_null v1) (as_null v2))
  | T.Bool -> fun v1 v2 -> Bool (fbool (as_bool v1) (as_bool v2))
  | t -> ord_relop fnat fint fwords ffloat fchar ftext fblob t

let eq_prim =
  eq_relop Nat.eq  Int.eq (Nat8.eq, Nat16.eq, Nat32.eq, Nat64.eq, Int_8.eq, Int_16.eq, Int_32.eq, Int_64.eq) Float.eq (=) (=) (=) (=) (=)

(* Follows the structure of `shared` in mo_type/type.ml *)
let structural_equality t =
  let rec go t =
    match t with
    | T.Var _ | T.Pre | T.Non | T.Async _ | T.Mut _ | T.Weak _ -> assert false
    | T.Any -> fun v1 v2 -> Bool true
    | T.Prim T.Error
    | T.Prim T.Region -> assert false
    | T.Prim p -> eq_prim p
    | T.Con (c, ts) -> (
        match Mo_types.Cons.kind c with
        | T.Abs _ -> assert false
        | T.Def (_, t) -> go (T.open_ ts t) (* TBR this may fail to terminate *)
        )
    | T.Array t ->
        fun v1 v2 ->
          let eq_elem = go t in
          let v1 = as_array v1 in
          let v2 = as_array v2 in
          Bool (
            Array.length v1 = Array.length v2 &&
            Lib.Array.for_all2 (fun x y -> as_bool (eq_elem x y)) v1 v2
          )
    | T.Opt t -> (
        fun v1 v2 ->
          match (v1, v2) with
          | Null, Null -> Bool true
          | Null, Opt _
          | Opt _, Null -> Bool false
          | Opt v1, Opt v2 -> go t v1 v2
          | _, _ -> assert false )
    | T.Tup ts ->
        fun v1 v2 ->
          let v1 = as_tup v1 in
          let v2 = as_tup v2 in
          let rec go_inner ts v1 v2 =
            match (ts, v1, v2) with
            | [], [], [] -> true
            | t :: ts, v1 :: v1s, v2 :: v2s ->
                as_bool (go t v1 v2) && go_inner ts v1s v2s
            | _ -> assert false
          in
          Bool (go_inner ts v1 v2)
    | T.Obj (s, fs, _) -> (
        match s with
        | T.Actor ->
            fun v1 v2 ->
              (match (v1, v2) with
               | Blob s1, Blob s2 -> Bool (s1 = s2)
               | _, _ -> Bool (v1 == v2) (* HACK *))
        | T.Module | T.Mixin | T.Memory -> assert false
        | T.Object ->
            fun v1 v2 ->
              let v1 = as_obj v1 in
              let v2 = as_obj v2 in
              Bool
                (List.for_all
                   (fun f -> as_bool (go f.T.typ (Env.find f.T.lab v1) (Env.find f.T.lab v2)))
                   fs))
    | T.Variant fs ->
        fun v1 v2 ->
          let l1, v1 = as_variant v1 in
          let l2, v2 = as_variant v2 in
          if l1 <> l2 then Bool false
          else
            go (List.find (fun f -> f.T.lab = l1) fs).T.typ v1 v2
    | T.Func (s, c, tbs, ts1, ts2) ->
       assert (T.is_shared_sort s);
      (fun v1 v2 -> match v1, v2 with
       | Tup [Blob _; Text _], Tup [Blob _; Text _] -> Bool (v1 = v2) (* public methods *)
       | Func _, Tup [Blob _; Text _]
       | Tup [Blob _; Text _], Func _ -> assert false; (* mixed, cannot determine equality *)
       | Func _, Func _ -> Bool (v1 == v2)  (* both internal, HACK *)
       | _ -> failwith "illegal shared function")
    | T.Named (n, t1) -> go t1
  in
  go t

let relop op t =
  match t with
  | T.Prim p ->
    (match op with
    | EqOp -> eq_prim p
    | NeqOp -> eq_relop Nat.ne Int.ne (Nat8.ne, Nat16.ne, Nat32.ne, Nat64.ne, Int_8.ne, Int_16.ne, Int_32.ne, Int_64.ne) Float.ne (<>) (<>) (<>) (<>) (<>) p
    | LtOp -> ord_relop Nat.lt Int.lt (Nat8.lt, Nat16.lt, Nat32.lt, Nat64.lt, Int_8.lt, Int_16.lt, Int_32.lt, Int_64.lt) Float.lt (<) (<) (<) p
    | GtOp -> ord_relop Nat.gt Int.gt (Nat8.gt, Nat16.gt, Nat32.gt, Nat64.gt, Int_8.gt, Int_16.gt, Int_32.gt, Int_64.gt) Float.gt (>) (>) (>) p
    | LeOp -> ord_relop Nat.le Int.le (Nat8.le, Nat16.le, Nat32.le, Nat64.le, Int_8.le, Int_16.le, Int_32.le, Int_64.le) Float.le (<=) (<=) (<=) p
    | GeOp -> ord_relop Nat.ge Int.ge (Nat8.ge, Nat16.ge, Nat32.ge, Nat64.ge, Int_8.ge, Int_16.ge, Int_32.ge, Int_64.ge) Float.ge (>=) (>=) (>=) p
    )
  | T.Non -> impossible
  | t when op = EqOp && T.shared t ->
    structural_equality t
  | t when op = NeqOp && T.shared t ->
    fun v1 v2 -> Bool (not (as_bool (structural_equality t v1 v2)))
  | _ -> raise (Invalid_argument "relop")


let has f op t = try ignore (f op t); true with Invalid_argument _ -> false
let has_unop op t = has unop op t
let has_binop op t = has binop op t
let has_relop op t = has relop op t

let type_unop op t = if t = T.nat then T.int else t
let type_binop op t = t
let type_relop op t = t
