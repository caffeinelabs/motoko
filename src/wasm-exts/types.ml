(*
This module originated as a copy of interpreter/syntax/types.ml in the
reference implementation.
With adjustments from memory 64.
*)

module I32 = Wasm.I32
module I64 = Wasm.I64

let rec map_filter f = function
| [] -> []
| x::xs ->
  match f x with
  | None -> map_filter f xs
  | Some y -> y :: map_filter f xs

(* Types *)

type value_type = I32Type | I64Type | F32Type | F64Type | V128Type
type index_type = I32IndexType | I64IndexType
type elem_type = FuncRefType
type stack_type = value_type list
type func_type = FuncType of stack_type * stack_type

type 'a limits = {min : 'a; max : 'a option}
type mutability = Immutable | Mutable
type table_type = TableType of Int32.t limits * elem_type
                | HugeTableType of Int64.t limits * elem_type
type memory_type = MemoryType of Int64.t limits * index_type
type global_type = GlobalType of value_type * mutability
type extern_type =
  | ExternFuncType of func_type
  | ExternTableType of table_type
  | ExternMemoryType of memory_type
  | ExternGlobalType of global_type

type pack_size = Pack8 | Pack16 | Pack32
type extension = SX | ZX


(* Attributes *)

let size = function
  | I32Type | F32Type -> 4
  | I64Type | F64Type -> 8
  | V128Type -> 16

let packed_size = function
  | Pack8 -> 1
  | Pack16 -> 2
  | Pack32 -> 4

let value_type_of_index_type = function
  | I32IndexType -> I32Type
  | I64IndexType -> I64Type


(* Subtyping *)

let match_limits ge lim1 lim2 =
  ge lim1.min lim2.min &&
  match lim1.max, lim2.max with
  | _, None -> true
  | None, Some _ -> false
  | Some i, Some j -> ge j i

let match_func_type ft1 ft2 =
  ft1 = ft2

let match_table_type tt1 tt2 = match tt1, tt2 with
  | TableType (lim1, et1), TableType (lim2, et2) ->
    et1 = et2 && match_limits I32.ge_u lim1 lim2
  | HugeTableType (lim1, et1), HugeTableType (lim2, et2) ->
    et1 = et2 && match_limits I64.ge_u lim1 lim2
  | TableType ({min; max}, et1), HugeTableType (lim2, et2) ->
    let lim1 = {min = Int64.of_int32 min; max = Option.map Int64.of_int32 max} in
    et1 = et2 && match_limits I64.ge_u lim1 lim2
  | HugeTableType (lim1, et1), TableType ({min; max}, et2) ->
    let lim2 = {min = Int64.of_int32 min; max = Option.map Int64.of_int32 max} in
    et1 = et2 && match_limits I64.ge_u lim1 lim2

let match_memory_type (MemoryType (lim1, it1)) (MemoryType (lim2, it2)) =
  it1 = it2 && match_limits I64.ge_u lim1 lim2

let match_global_type gt1 gt2 =
  gt1 = gt2

let match_extern_type et1 et2 =
  match et1, et2 with
  | ExternFuncType ft1, ExternFuncType ft2 -> match_func_type ft1 ft2
  | ExternTableType tt1, ExternTableType tt2 -> match_table_type tt1 tt2
  | ExternMemoryType mt1, ExternMemoryType mt2 -> match_memory_type mt1 mt2
  | ExternGlobalType gt1, ExternGlobalType gt2 -> match_global_type gt1 gt2
  | _, _ -> false


(* Filters *)

let funcs =
  map_filter (function ExternFuncType t -> Some t | _ -> None)
let tables =
  map_filter (function ExternTableType t -> Some t | _ -> None)
let memories =
  map_filter (function ExternMemoryType t -> Some t | _ -> None)
let globals =
  map_filter (function ExternGlobalType t -> Some t | _ -> None)

(* String conversion *)

let string_of_value_type = function
  | I32Type -> "i32"
  | I64Type -> "i64"
  | F32Type -> "f32"
  | F64Type -> "f64"
  | V128Type -> "v128"

let string_of_value_types = function
  | [t] -> string_of_value_type t
  | ts -> "[" ^ String.concat " " (List.map string_of_value_type ts) ^ "]"

let string_of_elem_type = function
  | FuncRefType -> "funcref"

let string_of_limits to_string {min; max} =
  to_string min ^
  (match max with None -> "" | Some n -> " " ^ to_string n)

let string_of_memory_type = function
  | MemoryType (lim, it) ->
    string_of_value_type (value_type_of_index_type it) ^
    " " ^ string_of_limits I64.to_string_u lim

let string_of_table_type = function
  | TableType (lim, t) -> string_of_limits I32.to_string_u lim ^ " " ^
                          string_of_elem_type t
  | HugeTableType (lim, t) -> string_of_limits I64.to_string_u lim ^ " " ^
                              string_of_elem_type t

let string_of_global_type = function
  | GlobalType (t, Immutable) -> string_of_value_type t
  | GlobalType (t, Mutable) -> "(mut " ^ string_of_value_type t ^ ")"

let string_of_stack_type ts =
  "[" ^ String.concat " " (List.map string_of_value_type ts) ^ "]"

let string_of_func_type (FuncType (ins, out)) =
  string_of_stack_type ins ^ " -> " ^ string_of_stack_type out

let string_of_extern_type = function
  | ExternFuncType ft -> "func " ^ string_of_func_type ft
  | ExternTableType tt -> "table " ^ string_of_table_type tt
  | ExternMemoryType mt -> "memory " ^ string_of_memory_type mt
  | ExternGlobalType gt -> "global " ^ string_of_global_type gt
