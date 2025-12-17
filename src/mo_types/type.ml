open Field_sources

(* Representation *)
type lab = string
type var = string
type name = string

type control =
  | Returns        (* regular local function or one-shot shared function *)
  | Promises       (* shared function producing a future value upon call *)
  | Replies        (* (IR only): responds asynchronously using `reply` *)

type obj_sort =
   Object
 | Actor
 | Mixin
 | Module
 | Memory          (* (codegen only): stable memory serialization format *)

type async_sort = Fut | Cmp
type await_sort = AwaitFut of bool | AwaitCmp
type shared_sort = Query | Write | Composite
type 'a shared = Local | Shared of 'a
type func_sort = shared_sort shared
type eff = Triv | Await

type prim =
  | Null
  | Bool
  | Nat
  | Nat8
  | Nat16
  | Nat32
  | Nat64
  | Int
  | Int8
  | Int16
  | Int32
  | Int64
  | Float
  | Char
  | Text
  | Blob (* IR use: Packed representation, vec u8 IDL type *)
  | Error
  | Principal
  | Region

type t = typ
and typ =
  | Var of var * int                          (* variable *)
  | Con of con * typ list                     (* constructor *)
  | Prim of prim                              (* primitive *)
  | Obj of obj_sort * field list              (* object *)
  | Variant of field list                     (* variant *)
  | Array of typ                              (* array *)
  | Opt of typ                                (* option *)
  | Tup of typ list                           (* tuple *)
  | Func of func_sort * control * bind list * typ list * typ list  (* function *)
  | Async of async_sort * scope * typ                      (* future *)
  | Mut of typ                                (* mutable type *)
  | Any                                       (* top *)
  | Non                                       (* bottom *)
  | Typ of con                                (* type (field of module) *)
  | Named of name * typ                       (* named type *)
  | Weak of typ                               (* weak reference *)
  | Pre                                       (* pre-type *)

and scope = typ
and bind_sort = Scope | Type

and bind = {var : var; sort: bind_sort; bound : typ}
and src = {depr : string option; track_region : Source.region; region : Source.region}
and field = {lab : lab; typ : typ; src : src}

and con = kind Cons.t
and kind =
  | Def of bind list * typ
  | Abs of bind list * typ

let empty_src = {depr = None; track_region = Source.no_region; region = Source.no_region}

(* Stable signatures *)
type stab_sig =
  | Single of field list
  | PrePost of ((* required *) bool * field) list * field list

(* Efficient comparison *)
let tag_prim = function
  | Null -> 0
  | Bool -> 1
  | Nat -> 2
  | Nat8 -> 3
  | Nat16 -> 4
  | Nat32 -> 5
  | Nat64 -> 6
  | Int -> 7
  | Int8 -> 8
  | Int16 -> 9
  | Int32 -> 10
  | Int64 -> 11
  | Float -> 12
  | Char -> 13
  | Text -> 14
  | Blob -> 15
  | Error -> 16
  | Principal -> 17
  | Region -> 18

let tag_func_sort = function
  | Local -> 0
  | Shared Write -> 1
  | Shared Query -> 2
  | Shared Composite -> 3

let tag_obj_sort = function
  | Object -> 0
  | Module -> 1
  | Actor -> 2
  | Memory -> 3
  | Mixin -> 4

let tag_control = function
  | Returns -> 0
  | Promises -> 1
  | Replies -> 2

let tag = function
  | Prim _ -> 0
  | Var _ -> 1
  | Con _ -> 2
  | Array _ -> 3
  | Tup _ -> 4
  | Func _ -> 5
  | Opt _ -> 6
  | Async _ -> 7
  | Obj _ -> 8
  | Variant _ -> 9
  | Mut _ -> 10
  | Any -> 11
  | Non -> 12
  | Pre -> 13
  | Typ _ -> 14
  | Named _ -> 15
  | Weak _ -> 16

let compare_prim p1 p2 =
  let d = tag_prim p1 - tag_prim p2 in
  if d > 0 then 1 else if d < 0 then -1 else 0

let compare_control c1 c2 =
  let d = tag_control c1 - tag_control c2 in
  if d > 0 then 1 else if d < 0 then -1 else 0

let compare_async_sort s1 s2 =
  match s1, s2 with
  | Fut, Fut
  | Cmp, Cmp -> 0
  | Fut, Cmp -> -1
  | Cmp, Fut -> 1

let compare_obj_sort s1 s2 =
  let d = tag_obj_sort s1 - tag_obj_sort s2 in
  if d > 0 then 1 else if d < 0 then -1 else 0

let compare_func_sort s1 s2 =
  let d = tag_func_sort s1 - tag_func_sort s2 in
  if d > 0 then 1 else if d < 0 then -1 else 0

let compare_bind_sort s1 s2 =
  match s1, s2 with
  | Type, Type
  | Scope, Scope -> 0
  | Type, Scope -> -1
  | Scope, Type -> 1

let compare_src s1 s2 =
  match (s1.depr, s2.depr) with
  | None, None -> 0
  | Some d1, Some d2 -> String.compare d1 d2
  | None, Some _ -> -1
  | _ -> 1

let rec compare_typ (t1 : typ) (t2 : typ) =
  if t1 == t2 then 0
  else match (t1, t2) with
  | Prim p1, Prim p2 ->
    compare_prim p1 p2
  | Var (s1, i1), Var (s2, i2) ->
    (match Int.compare i1 i2 with
     | 0 -> String.compare s1 s2
     | ord -> ord)
  | Con (c1, ts1), Con (c2, ts2) ->
    (match Cons.compare c1 c2 with
     | 0 -> compare_typs ts1 ts2
     | ord -> ord)
  | Array t1, Array t2 ->
    compare_typ t1 t2
  | Tup ts1, Tup ts2 ->
    compare_typs ts1 ts2
  | Func (s1, c1, tbs1, ts11, ts12),
    Func (s2, c2, tbs2, ts21, ts22) ->
    (match compare_func_sort s1 s2 with
     | 0 ->
       (match compare_control c1 c2 with
        | 0 ->
          (match compare_tbs tbs1 tbs2 with
           | 0 ->
             (match compare_typs ts11 ts21 with
             | 0 -> compare_typs ts12 ts22
             | ord -> ord)
           | ord -> ord)
        | ord -> ord)
     | ord -> ord)
  | Opt t1, Opt t2 -> compare_typ t1 t2
  | Async (s1, t11, t12) , Async (s2, t21, t22) ->
    (match compare_async_sort s1 s2 with
     | 0 -> (match compare_typ t11 t21 with
             | 0 -> compare_typ t12 t22
             | ord -> ord)
     | ord -> ord)
  | Obj (s1, fs1), Obj (s2, fs2) ->
    (match compare_obj_sort s1 s2 with
     | 0 -> compare_flds fs1 fs2
     | ord -> ord)
  | Variant fs1, Variant fs2 ->
    compare_flds fs1 fs2
  | Mut t1, Mut t2 ->
    compare_typ t1 t2
  | Any, Any
  | Non, Non
  | Pre, Pre -> 0
  | Typ c1, Typ c2 -> Cons.compare c1 c2
  | Named (n1, t1), Named (n2, t2) ->
    (match String.compare n1 n2 with
     | 0 -> compare_typ t1 t2
     | ord -> ord)
  | _ -> Int.compare (tag t1) (tag t2)

and compare_tb tb1 tb2 =
  match String.compare tb1.var tb2.var with
  | 0 ->
    (match compare_typ tb1.bound tb2.bound with
     | 0 -> compare_bind_sort tb1.sort tb2.sort
     | ord -> ord)
  | ord ->  ord

and compare_tbs tbs1 tbs2 =
  match (tbs1, tbs2) with
  | [], [] -> 0
  | [], (_::_) -> -1
  | (_::_, []) -> 1
  | (tb1::tbs1, tb2 :: tbs2) ->
    (match compare_tb tb1 tb2 with
     | 0 -> compare_tbs tbs1 tbs2
     | ord -> ord)

and compare_fld fld1 fld2 =
  match String.compare fld1.lab fld2.lab with
  | 0 ->
    (match compare_typ fld1.typ fld2.typ with
     | 0 -> compare_src fld1.src fld2.src
     | ord -> ord)
  | ord -> ord

and compare_flds flds1 flds2 =
  match (flds1, flds2) with
  | [], [] -> 0
  | [], (_::_) -> -1
  | (_::_, []) -> 1
  | (fld1::flds1, fld2 :: flds2) ->
    (match compare_fld fld1 fld2 with
     | 0 -> compare_flds flds1 flds2
     | ord -> ord)

and compare_typs ts1 ts2 =
  match (ts1, ts2) with
  | [], [] -> 0
  | [], (_::_) -> -1
  | (t1::ts1, t2 :: ts2) ->
    (match compare_typ t1 t2 with
     | 0 -> compare_typs ts1 ts2
     | ord -> ord)
  | _ -> 1


let compare_rel (t1, t2) (u1, u2) =
  match compare_typ (t1 : typ) (u1 : typ) with
  | 0 -> compare_typ (t2 : typ) (u2 : typ)
  | ord -> ord

(* Syntactic orderings *)

module Ord = struct
  type t = typ
  let compare = compare_typ
end

module OrdPair = struct
  type t = typ * typ
  let compare = compare_rel
end

(* Function sorts *)

let is_shared_sort sort = sort <> Local

(* Constructors *)

let set_kind c k =
  match Cons.kind c with
  | Abs (_, Pre) -> Cons.unsafe_set_kind c k
  | _ -> raise (Invalid_argument "set_kind")

module ConEnv = Env.Make(struct type t = con let compare = Cons.compare end)

module ConSet = ConEnv.Dom


(* Field ordering

   NOTE: Keep in sync with mo_frontend/typing.ml:compare_pat_field *)

let compare_field f1 f2 =
  match f1,f2 with
  | {lab = l1; typ = Typ _; _}, {lab = l2; typ = Typ _; _} -> compare l1 l2
  | {lab = l1; typ = Typ _; _}, {lab = l2; typ = _; _} -> -1
  | {lab = l1; typ = _; _}, {lab = l2; typ = Typ _; _} -> 1
  | {lab = l1; typ = _; _}, {lab = l2; typ = _; _} -> compare l1 l2


(* Shorthands *)

let unit = Tup []
let bool = Prim Bool
let nat = Prim Nat
let nat32 = Prim Nat32
let nat64 = Prim Nat64
let int = Prim Int
let text = Prim Text
let blob = Prim Blob
let error = Prim Error
let char = Prim Char
let principal = Prim Principal
let region = Prim Region


let fields flds =
  List.sort compare_field
    (List.map (fun (lab, typ) -> {lab; typ; src = empty_src}) flds)

let obj sort flds =
  Obj (sort, fields flds)

let sum flds =
  Variant (fields flds)

let throwErrorCodes = List.sort compare_field [
  { lab = "canister_reject"; typ = unit; src = empty_src}
]

let call_error = Obj(Object,[{ lab = "err_code"; typ = Prim Nat32; src = empty_src}])

let catchErrorCodes = List.sort compare_field (
  throwErrorCodes @ [
    { lab = "system_fatal"; typ = unit; src = empty_src};
    { lab = "system_transient"; typ = unit; src = empty_src};
    { lab = "destination_invalid"; typ = unit; src = empty_src};
    { lab = "canister_error"; typ = unit; src = empty_src};
    { lab = "system_unknown"; typ = unit; src = empty_src};
    { lab = "future"; typ = Prim Nat32; src = empty_src};
    { lab = "call_error"; typ = call_error; src = empty_src};
  ])

let throw = Prim Error
let catch = Prim Error

(* Shared call context *)

let caller = principal
let ctxt = Obj (Object,[{ lab = "caller"; typ = caller; src = empty_src}])

let prim = function
  | "Null" -> Null
  | "Bool" -> Bool
  | "Nat" -> Nat
  | "Nat8" -> Nat8
  | "Nat16" -> Nat16
  | "Nat32" -> Nat32
  | "Nat64" -> Nat64
  | "Int" -> Int
  | "Int8" -> Int8
  | "Int16" -> Int16
  | "Int32" -> Int32
  | "Int64" -> Int64
  | "Float" -> Float
  | "Char" -> Char
  | "Text" -> Text
  | "Blob" -> Blob
  | "Error" -> Error
  | "Principal" -> Principal
  | "Region" -> Region
  | s -> raise (Invalid_argument ("Type.prim: " ^ s))

let seq = function [t] -> t | ts -> Tup ts

let codom c to_scope ts2 =  match c with
  | Promises -> Async (Fut, to_scope(), seq ts2)
  | Returns -> seq ts2
  | Replies -> Tup []

(* Coercions *)

let iter_obj t =
  Obj (Object,
    [{lab = "next"; typ = Func (Local, Returns, [], [], [Opt t]); src = empty_src}])


(* Shifting *)

let rec shift i n t =
  match t with
  | Prim _ -> t
  | Var (s, j) -> Var (s, if j < i then j else j + n)
  | Con (c, ts) -> Con (c, List.map (shift i n) ts)
  | Array t -> Array (shift i n t)
  | Tup ts -> Tup (List.map (shift i n) ts)
  | Func (s, c, tbs, ts1, ts2) ->
    let i' = i + List.length tbs in
    Func (s, c, List.map (shift_bind i' n) tbs, List.map (shift i' n) ts1, List.map (shift i' n) ts2)
  | Opt t -> Opt (shift i n t)
  | Async (s, t1, t2) -> Async (s, shift i n t1, shift i n t2)
  | Obj (s, fs) -> Obj (s, List.map (shift_field n i) fs)
  | Variant fs -> Variant (List.map (shift_field n i) fs)
  | Mut t -> Mut (shift i n t)
  | Any -> Any
  | Non -> Non
  | Pre -> Pre
  | Typ c -> Typ c
  | Named (name, t) -> Named (name, shift i n t)
  | Weak t -> Weak (shift i n t)

and shift_bind i n tb =
  {tb with bound = shift i n tb.bound}

and shift_field i n {lab; typ; src} =
  {lab; typ = shift i n typ; src}

(*
and shift_kind i n k =
  match k with
  | Def (tbs, t) ->
    let i' = i + List.length tbs in
    Def (List.map (shift_bind i' n) tbs, shift i' n t)
  | Abs (tbs, t) ->
    let i' = i + List.length tbs in
    Abs (List.map (shift_bind i' n) tbs, shift i' n t)
 *)


(* First-order substitution *)

let rec subst sigma t =
  if sigma = ConEnv.empty then t else
  match t with
  | Prim _
  | Var _ -> t
  | Con (c, ts) ->
    (match ConEnv.find_opt c sigma with
    | Some t -> assert (List.length ts = 0); t
    | None -> Con (c, List.map (subst sigma) ts)
    )
  | Array t -> Array (subst sigma t)
  | Tup ts -> Tup (List.map (subst sigma) ts)
  | Func (s, c, tbs, ts1, ts2) ->
    let sigma' = ConEnv.map (shift 0 (List.length tbs)) sigma in
    Func (s, c, List.map (subst_bind sigma') tbs,
          List.map (subst sigma') ts1, List.map (subst sigma') ts2)
  | Opt t -> Opt (subst sigma t)
  | Async (s, t1, t2) -> Async (s, subst sigma t1, subst sigma t2)
  | Obj (s, fs) -> Obj (s, List.map (subst_field sigma) fs)
  | Variant fs -> Variant (List.map (subst_field sigma) fs)
  | Mut t -> Mut (subst sigma t)
  | Any -> Any
  | Non -> Non
  | Pre -> Pre
  | Typ c -> Typ c (* NB: this is incorrect unless we ensure dom(sigma) \cap FV(c.kind) = {}
                      For now, we could do this by ensuring all type definitions are closed,
                      in particular, type components defined within the scope of an enclosing
                      type parameter cannot mention that parameter
                      (but can mention other (closed) type constructors).
                    *)
  | Named (name, t) -> Named (name, subst sigma t)
  | Weak t -> Weak (subst sigma t)

and subst_bind sigma tb =
  { tb with bound = subst sigma tb.bound}

and subst_field sigma {lab; typ; src} =
  {lab; typ = subst sigma typ; src}

(*
and subst_kind sigma k =
  match k with
  | Def (tbs, t) ->
    let sigma' = ConEnv.map (shift 0 (List.length tbs)) sigma in
    Def (List.map (subst_bind sigma') tbs, subst sigma' t)
  | Abs (tbs, t) ->
    let sigma' = ConEnv.map (shift 0 (List.length tbs)) sigma in
    Abs (List.map (subst_bind sigma') tbs, subst sigma' t)
 *)

(* Handling binders *)

let close cs t =
  if cs = [] then t else
  let ts = List.mapi (fun i c -> Var (Cons.name c, i)) cs in
  let sigma = List.fold_right2 ConEnv.add cs ts ConEnv.empty in
  subst sigma t

let close_binds cs tbs =
  if cs = [] then tbs else
  List.map (fun tb -> { tb with bound = close cs tb.bound })  tbs

let rec open' i ts t =
  match t with
  | Prim _ -> t
  | Var (_, j) -> if j < i then t else List.nth ts (j - i)
  | Con (c, ts') -> Con (c, List.map (open' i ts) ts')
  | Array t -> Array (open' i ts t)
  | Tup ts' -> Tup (List.map (open' i ts) ts')
  | Func (s, c, tbs, ts1, ts2) ->
    let i' = i + List.length tbs in
    Func (s, c, List.map (open_bind i' ts) tbs, List.map (open' i' ts) ts1, List.map (open' i' ts) ts2)
  | Opt t -> Opt (open' i ts t)
  | Async (s, t1, t2) -> Async (s, open' i ts t1, open' i ts t2)
  | Obj (s, fs) -> Obj (s, List.map (open_field i ts) fs)
  | Variant fs -> Variant (List.map (open_field i ts) fs)
  | Mut t -> Mut (open' i ts t)
  | Any -> Any
  | Non -> Non
  | Pre -> Pre
  | Typ c -> Typ c
  | Named (name, t) -> Named (name, open' i ts t)
  | Weak t -> Weak (open' i ts t)

and open_bind i ts tb  =
  {tb with bound = open' i ts tb.bound}

and open_field i ts {lab; typ; src} =
  {lab; typ = open' i ts typ; src}

(*
and open_kind i ts k =
  match k with
  | Def (tbs, t) ->
    let i' = i + List.length tbs in
    Def (List.map (open_bind i' ts) tbs, open' i' ts t)
  | Abs (tbs, t) ->
    let i' = i + List.length tbs in
    Abs (List.map (open_bind i' ts) tbs, open' i' ts t)
*)

let open_ ts t =
  if ts = [] then t else
  open' 0 ts t

let open_binds tbs =
  if tbs = [] then [] else
  let cs = List.map (fun {var; _} -> Cons.fresh var (Abs ([], Pre))) tbs in
  let ts = List.map (fun c -> Con (c, [])) cs in
  let ks = List.map (fun {bound; _} -> Abs ([], open_ ts bound)) tbs in
  List.iter2 set_kind cs ks;
  ts

let open_binds_unbounded tbs =
  if tbs = [] then [] else
  let cs = List.map (fun {var; _} -> Cons.fresh var (Abs ([], Pre))) tbs in
  let ts = List.map (fun c -> Con (c, [])) cs in
  let ks = List.map (fun {bound; _} -> Abs ([], Any)) tbs in
  List.iter2 set_kind cs ks;
  ts


(* Normalization and Classification *)

let reduce tbs t ts =
  assert (List.length ts = List.length tbs);
  open_ ts t

let rec normalize = function
  | Con (con, ts) as t ->
    (match Cons.kind con with
    | Def (tbs, t) -> normalize (reduce tbs t ts)
    | _ -> t
    )
  | Mut t -> Mut (normalize t)
  | Named (_, t) -> normalize t
  | t -> t

let rec promote = function
  | Con (con, ts) ->
    let Def (tbs, t) | Abs (tbs, t) = Cons.kind con
    in promote (reduce tbs t ts)
  | Named (_, t) -> promote t
  | t -> t

(* Projections *)

let is_non = function Non -> true | _ -> false
let is_prim p = function Prim p' -> p = p' | _ -> false
let is_obj = function Obj _ -> true | _ -> false
let is_module = function Obj (Module, _) -> true | _ -> false
let is_variant = function Variant _ -> true | _ -> false
let is_array = function Array _ -> true | _ -> false
let is_opt = function Opt _ -> true | _ -> false
let is_tup = function Tup _ -> true | _ -> false
let is_unit = function Tup [] -> true | _ -> false
let is_pair = function Tup [_; _] -> true | _ -> false
let is_func = function Func _ -> true | _ -> false
let is_async = function Async _ -> true | _ -> false
let is_fut = function Async (Fut, _, _) -> true | _ -> false
let is_cmp = function Async (Cmp, _, _) -> true | _ -> false
let is_mut = function Mut _ -> true | _ -> false
let is_typ = function Typ _ -> true | _ -> false
let is_con = function Con _ -> true | _ -> false
let is_var = function Var _ -> true | _ -> false

let invalid s = raise (Invalid_argument ("Type." ^ s))

let as_prim p = function Prim p' when p = p' -> () | _ -> invalid "as_prim"
let as_obj = function Obj (s, tfs) -> s, tfs | _ -> invalid "as_obj"
let as_array = function Array t -> t | _ -> invalid "as_array"
let as_opt = function Opt t -> t | _ -> invalid "as_opt"
let as_variant = function Variant fs -> fs | _ -> invalid "as_variant"
let as_tup = function Tup ts -> ts | _ -> invalid "as_tup"
let as_unit = function Tup [] -> () | _ -> invalid "as_unit"
let as_pair = function Tup [t1; t2] -> t1, t2 | _ -> invalid "as_pair"
let as_func = function Func (s, c, tbs, ts1, ts2) -> s, c, tbs, ts1, ts2 | _ -> invalid "as_func"
let as_async = function Async (s, t1, t2) -> (s, t1, t2) | _ -> invalid "as_async"
let as_mut = function Mut t -> t | _ -> invalid "as_mut"
let as_immut = function Mut t -> t | t -> t
let as_typ = function Typ c -> c | _ -> invalid "as_typ"
let as_con = function Con (c, ts) -> c, ts | _ -> invalid "as_con"

let as_seq t =
  match normalize t with
  | Tup ts -> ts
  | t -> [t]

let seq_of_tup t =
  match normalize t with
  | Tup ts -> ts
  | t -> invalid "seq_of_tup"

let arity t =
  match normalize t with
  | Tup ts -> List.length ts
  | t -> 1

let as_prim_sub p t = match promote t with
  | Prim p' when p = p' -> ()
  | Non -> ()
  | _ -> invalid "as_prim_sub"
let as_obj_sub ls t = match promote t with
  | Obj (s, tfs) -> s, tfs
  | Non -> Object, List.map (fun l -> {lab = l; typ = Non; src = empty_src}) ls
  | _ -> invalid "as_obj_sub"
let as_variant_sub l t = match promote t with
  | Variant tfs -> tfs
  | Non -> [{lab = l; typ = Non; src = empty_src}]
  | _ -> invalid "as_variant_sub"
let as_array_sub t = match promote t with
  | Array t -> t
  | Non -> Non
  | _ -> invalid "as_array_sub"
let as_opt_sub t = match promote t with
  | Opt t -> t
  | Prim Null -> Non
  | Non -> Non
  | _ -> invalid "as_opt_sub"
let as_tup_sub n t = match promote t with
  | Tup ts -> ts
  | Non -> Lib.List.make n Non
  | _ -> invalid "as_tup_sub"
let as_unit_sub t = match promote t with
  | Tup []
  | Non -> ()
  | _ -> invalid "as_unit_sub"
let as_pair_sub t = match promote t with
  | Tup [t1; t2] -> t1, t2
  | Non -> Non, Non
  | _ -> invalid "as_pair_sub"
let as_func_sub default_s default_arity t = match promote t with
  | Func (s, c, tbs, ts1, ts2) ->
    s, tbs, ts1, codom c (fun () -> Var((List.hd tbs).var, 0)) ts2
  | Non -> default_s, Lib.List.make default_arity {var = "X"; sort = Type; bound = Any}, [Any], Non
  | _ -> invalid "as_func_sub"
let as_mono_func_sub t = match promote t with
  | Func (_, _, [], ts1, ts2) -> seq ts1, seq ts2
  | Non -> Any, Non
  | _ -> invalid "as_mono_func_sub"
let as_async_sub s default_scope t = match promote t with
  | Async (s0, t1, t2) when s = s0 -> (t1, t2)
  | Non -> default_scope, Non (* TBR *)
  | _ -> invalid "as_async_sub"

let as_weak_sub t = match promote t with
  | Weak t -> t
  | Non -> Non
  | _ -> invalid "as_weak_sub"

let is_immutable_obj obj_type =
  let _, fields = as_obj_sub [] obj_type in
  List.for_all (fun f -> not (is_mut f.typ)) fields

let find_val_field_opt l tfs =
  let is_lab = function {typ = Typ _; _} -> false | {lab; _} -> lab = l in
  match List.find_opt is_lab tfs with
  | Some tf -> Some tf
  | None -> None

let lookup_val_field_opt l tfs =
  match find_val_field_opt l tfs with
  | Some tf -> Some tf.typ
  | None -> None

let lookup_typ_field_opt l tfs =
  let is_lab = function {typ = Typ _; lab; _} -> lab = l | _ -> false in
  match List.find_opt is_lab tfs with
  | Some {lab = _; typ = Typ c; src} -> Some c
  | _ -> None

let lookup_val_field l tfs =
  match lookup_val_field_opt l tfs with
  | Some t -> t
  | None -> invalid "lookup_val_field"

let lookup_typ_field l tfs =
  match lookup_typ_field_opt l tfs with
  | Some c -> c
  | _ -> invalid "lookup_typ_field"


let lookup_val_deprecation l tfs =
  let is_lab = function {typ = Typ _; _} -> false | {lab; _} -> lab = l in
  match List.find_opt is_lab tfs with
  | Some {src = {depr; _}; _} -> depr
  | None -> invalid "lookup_val_deprecation"

let lookup_typ_deprecation l tfs =
  let is_lab = function {typ = Typ _; lab; _} -> lab = l | _ -> false in
  match List.find_opt is_lab tfs with
  | Some {src = {depr; _}; _} -> depr
  | _ -> invalid "lookup_typ_deprecation"


(* Span *)

let rec span = function
  | Var _ | Pre -> assert false
  | Con _ as t -> span (promote t)
  | Prim Null -> Some 1
  | Prim Bool -> Some 2
  | Prim (Nat | Int | Float | Text | Blob | Error | Principal | Region) -> None
  | Prim (Nat8 | Int8) -> Some 0x100
  | Prim (Nat16 | Int16) -> Some 0x10000
  | Prim (Nat32 | Int32 | Nat64 | Int64 | Char) -> None  (* for all practical purposes *)
  | Obj _ | Tup _ | Async _ -> Some 1
  | Variant fs -> Some (List.length fs)
  | Array _ | Func _ | Any -> None
  | Opt _ -> Some 2
  | Mut t -> span t
  | Non -> Some 0
  | Typ _ -> Some 1
  | Named (_, t) -> span t
  | Weak t -> span t (* TBR *)


(* Collecting type constructors *)

(* Parameter `inTyp` controls whether to count a constructor, `c`,  that only occurs as an argument of
   `Typ` field, `Typ c`, but not in its own unfolding.
   Set to false to avoid emitting redundant bindings in stable signature.
*)

let rec cons' inTyp t cs =
  match t with
  | Var _ | Prim _ | Any | Non | Pre -> cs
  | Con (c, ts) ->
    List.fold_right (cons' inTyp) ts (cons_con inTyp c cs)
  | Opt t | Mut t | Array t | Weak t ->
    cons' inTyp t cs
  | Async (_, t1, t2) ->
    cons' inTyp t2 (cons' inTyp t1 cs)
  | Tup ts -> List.fold_right (cons' inTyp) ts cs
  | Func (s, c, tbs, ts1, ts2) ->
    let cs = List.fold_right (cons_bind inTyp) tbs cs in
    let cs = List.fold_right (cons' inTyp) ts1 cs in
    List.fold_right (cons' inTyp) ts2 cs
  | Obj (_, fs) | Variant fs ->
    List.fold_right (cons_field inTyp) fs cs
  | Typ c ->
    if inTyp then
      cons_con inTyp c cs
    else
      (* don't add c unless mentioned in Cons.kind c *)
      cons_kind' inTyp (Cons.kind c) cs
  | Named (_ , t) ->
    cons' inTyp t cs

and cons_con inTyp c cs =
  if ConSet.mem c cs
  then cs
  else cons_kind' inTyp (Cons.kind c) (ConSet.add c cs)

and cons_bind inTyp tb cs =
  cons' inTyp tb.bound cs

and cons_field inTyp {lab; typ; src} cs =
  cons' inTyp typ cs

and cons_kind' inTyp k cs =
  match k with
  | Def (tbs, t)
  | Abs (tbs, t) ->
    cons' inTyp t (List.fold_right (cons_bind inTyp) tbs cs)

let cons t = cons' true t ConSet.empty
let cons_kind k = cons_kind' true k ConSet.empty
let cons_typs ts = List.fold_left (fun acc t -> cons t |> ConSet.union acc) ConSet.empty ts

(* Checking for concrete types *)

module S = Set.Make (Ord)

(*
This check is a stop-gap measure until we have an IDL strategy that
allows polymorphic types, see #250. It is not what we desire for Motoko.
*)

let concrete t =
  let seen = ref S.empty in
  let rec go t =
    S.mem t !seen ||
    begin
      seen := S.add t !seen;
      match t with
      | Var _ | Pre -> assert false
      | Prim _ | Any | Non -> true
      | Con (c, ts) ->
        (match Cons.kind c with
        | Abs _ -> false
        | Def (_, t) -> go (open_ ts t) (* TBR this may fail to terminate *)
        )
      | Array t | Opt t | Mut t -> go t
      | Async (s, t1, t2) -> go t2 (* t1 is a phantom type *)
      | Tup ts -> List.for_all go ts
      | Obj (_, fs) | Variant fs -> List.for_all (fun f -> go f.typ) fs
      | Func (s, c, tbs, ts1, ts2) ->
        let ts = open_binds tbs in
        List.for_all go (List.map (open_ ts) ts1) &&
        List.for_all go (List.map (open_ ts) ts2)
      | Typ c -> (* assumes type defs are closed *)
        true (* so we can transmit actors with typ fields *)
      | Named (_, t) -> go t
      | Weak t -> go t
    end
  in go t

(* stable or shared *)
let serializable allow_mut t =
  let seen = ref S.empty in
  let rec go t =
    S.mem t !seen ||
    begin
      seen := S.add t !seen;
      match t with
      | Var _ | Pre -> assert false
      | Prim Error -> false
      | Prim Region -> allow_mut (* stable, but not shared *)
      | Any | Non | Prim _ | Typ _ -> true
      | Async _ -> false
      | Weak t ->
         !Mo_config.Flags.enhanced_orthogonal_persistence &&
         (* NB: Candid serialization doesn't preserve graph structure *)
         (* weak references are stable if content is stable *)
         allow_mut && go t
      | Mut t -> allow_mut && go t
      | Con (c, ts) ->
        (match Cons.kind c with
        | Abs _ -> false
        | Def (_, t) -> go (open_ ts t) (* TBR this may fail to terminate *)
        )
      | Array t | Opt t -> go t
      | Tup ts -> List.for_all go ts
      | Obj (s, fs) ->
        (match s with
         | Actor -> true
         | Module | Mixin -> false (* TODO(1452) make modules sharable *)
         | Object | Memory -> List.for_all (fun f -> go f.typ) fs)
      | Variant fs -> List.for_all (fun f -> go f.typ) fs
      | Func (s, c, tbs, ts1, ts2) -> is_shared_sort s
      | Named (n, t) -> go t
    end
  in go t

(* Find the first unshared subexpression in a type *)
let find_unshared t =
  let seen = ref S.empty in
  let rec go t =
    if S.mem t !seen then None else
    begin
      seen := S.add t !seen;
      match t with
      | Var _ | Pre -> assert false
      | Prim Error -> Some t
      | Any | Non | Prim _ | Typ _ -> None
      | Async _ | Mut _ | Weak _ -> Some t
      | Con (c, ts) ->
        (match Cons.kind c with
        | Abs _ -> None
        | Def (_, t) -> go (open_ ts t) (* TBR this may fail to terminate *)
        )
      | Array t | Opt t -> go t
      | Tup ts -> List.find_map go ts
      | Obj (s, fs) ->
        (match s with
         | Actor | Mixin -> None
         | Module -> Some t (* TODO(1452) make modules sharable *)
         | Object ->
           List.find_map (fun f -> go f.typ) fs
         | Memory -> assert false)
      | Variant fs -> List.find_map (fun f -> go f.typ) fs
      | Func (s, c, tbs, ts1, ts2) ->
        if is_shared_sort s
        then None
        else Some t
      | Named (n, t) -> go t
    end
  in go t

let is_shared_func typ =
  match promote typ with
  | Func (Shared _, _, _, _, _) -> true
  | _ -> false

let is_local_async_func typ =
  match promote typ with
  | Func
      (Local, Returns,
       { sort = Scope; _ }::_,
       _,
       [Async (Fut, Var (_ ,0), _)]) ->
    true
  | _ ->
    false

let shared t = serializable false t
let stable t = serializable true t


(* Forward declare
   TODO: haul string_of_typ before the lub/glb business, if possible *)
let str = ref (fun _ -> failwith "")

(* Aggregation of source fields, for use by the language server. *)
let src_field_updates = ref []
let src_field_map = ref (empty_srcs_tbl ())

(* Helper to perform source field updates to the given map. *)
let with_src_field_updates src_fields action =
  Fun.protect ~finally:(fun () -> src_field_map := empty_srcs_tbl ()) (fun () ->
    src_field_map := src_fields;
    action ())

(* Helper to aggregate field updates with an empty list, perform some check,
   commit field updates if the check holds, and restore the empty list. It
   will update the actions given to it in the source fields map . *)
let with_src_field_updates_predicate_general src_fields check passes =
  Fun.protect ~finally:(fun () -> src_field_updates := []; src_field_map := empty_srcs_tbl ()) (fun () ->
    src_field_updates := [];
    src_field_map := src_fields;
    let result = check () in
    (* Do not commit updates if the relation given by the predicate failed. *)
    if passes result then
      List.iter (fun f -> f ()) (List.rev !src_field_updates);
    result)

let with_src_field_updates_predicate src_fields predicate =
  with_src_field_updates_predicate_general src_fields predicate Fun.id

(* Updates to the source fields of the inputs depending on the given relation.
   If you use this function, you'll probably want to ensure the entire relation
   checking procedure is wrapped in [with_src_field_updates]. You'll probably
   want to use this with [combine]. *)
let add_src_field rel lubs glbs f1 f2 =
  if !Mo_config.Flags.typechecker_combine_srcs then
    let r1 = f1.src.track_region in
    let r2 = f2.src.track_region in
    let src_map = !src_field_map in
    (* Perhaps we could get away with just adding [r1] and [r2] to each other's
       tables, but we play safe here and overapproximate. *)
    let srcs =
      Source.Region_set.(if rel == lubs then union else inter)
        (get_srcs src_map r1)
        (get_srcs src_map r2)
    in
    Srcs_tbl.replace src_map r1 srcs;
    Srcs_tbl.replace src_map r2 srcs

(* Possibly stages an update to the source fields of the inputs in case the
   relation holds. If you use this function, you'll probably want to ensure the
   entire relation checking procedure is wrapped in
   [with_src_field_updates_predicate]. You'll probably want to use it when
   checking [sub] or [eq]. *)
let add_src_field_update is_rel rel eq tf1 tf2 =
  if !Mo_config.Flags.typechecker_combine_srcs && is_rel then
    let src_field_update () =
      let r1 = tf1.src.track_region in
      let r2 = tf2.src.track_region in
      let src_map = !src_field_map in
      (* Perhaps we could get away with just adding [r1] and [r2] to each
         other's tables, but we play safe here and overapproximate. *)
      let srcs = Source.Region_set.union (get_srcs src_map r1) (get_srcs src_map r2) in
      if rel == eq then
        Srcs_tbl.replace src_map r1 srcs;
      Srcs_tbl.replace src_map r2 srcs
    in
    src_field_updates := src_field_update :: !src_field_updates

(* Equivalence & Subtyping *)

exception PreEncountered

exception Undecided

module SS = Set.Make (OrdPair)

type compatibility = Compatible | Incompatible of explanation

and explanation =
  | IncompatibleTypes of context * typ * typ
  | MissingTag of context * lab * typ
  | UnexpectedTag of context * lab * typ
  | MissingField of context * lab * typ
  | UnexpectedField of context * lab * typ
  | FewerItems of context * string
  | MoreItems of context * string
  | PromotionToAny of context * typ
  | IncompatiblePrims of context * typ * typ
  | IncompatibleObjSorts of context * typ * typ
  | IncompatibleFuncSorts of context * typ * typ
  | IncompatibleBounds of context * typ * typ
  | IncompatibleFuncs of context * typ * typ
  | IncompatibleAsyncSorts of context * typ * typ
and context_item =
  | ConsType of con
  | NamedType of name
  | StableVariable of lab
  | Field of lab
and context = context_item list
let empty_context : context = []

module RelArg :
  sig
    type arg
    val sub : context -> arg (* ordinary subtyping, with loss of info *)
    val stable_sub : context -> arg (* stable subtyping, without loss of info*)
    val inc_depth : arg -> arg
    val push : context_item -> arg -> arg
    val is_stable_sub : arg -> bool
    val exceeds_max_depth : arg -> bool
    val context : arg -> context
    val false_with : arg -> explanation -> bool
    val explanation : arg -> explanation option
end
=
struct
  let max_depth = 10_000
  type arg = {
      is_stable_sub : bool;
      depth : int;
      context : context;
      error : explanation option ref;
    }
  let sub context = { context; depth = 0; is_stable_sub = false; error = ref None }
  let stable_sub context =  { (sub context) with is_stable_sub = true }
  let inc_depth arg =
    { arg with depth = arg.depth + 1 }
  let push context_item arg =
    { arg with context = context_item :: arg.context }
  let is_stable_sub arg = arg.is_stable_sub
  let exceeds_max_depth arg = arg.depth > max_depth
  let context arg = arg.context
  let false_with arg e = (arg.error := Some e; false)
  let explanation arg = !(arg.error)
end

let incompatible_types d t1 t2 =
  RelArg.false_with d (IncompatibleTypes (RelArg.context d, t1, t2))

let missing_tag d lab t =
  RelArg.false_with d (MissingTag (RelArg.context d, lab, t))

let unexpected_tag d lab t =
  RelArg.false_with d (UnexpectedTag (RelArg.context d, lab, t))

let missing_field d lab t =
  RelArg.false_with d (MissingField (RelArg.context d, lab, t))

let unexpected_field d lab t =
  RelArg.false_with d (UnexpectedField (RelArg.context d, lab, t))

let fewer_items d desc =
  RelArg.false_with d (FewerItems (RelArg.context d, desc))

let more_items d desc =
  RelArg.false_with d (MoreItems (RelArg.context d, desc))

let promotion_to_any d t =
  RelArg.false_with d (PromotionToAny (RelArg.context d, t))

let incompatible_prims d t1 t2 =
  RelArg.false_with d (IncompatiblePrims (RelArg.context d, t1, t2))

let incompatible_obj_sorts d t1 t2 =
  RelArg.false_with d (IncompatibleObjSorts (RelArg.context d, t1, t2))

let incompatible_func_sorts d t1 t2 =
  RelArg.false_with d (IncompatibleFuncSorts (RelArg.context d, t1, t2))

let incompatible_bounds d t1 t2 =
  RelArg.false_with d (IncompatibleBounds (RelArg.context d, t1, t2))

let incompatible_funcs d t1 t2 =
  RelArg.false_with d (IncompatibleFuncs (RelArg.context d, t1, t2))

let incompatible_async_sorts d t1 t2 =
  RelArg.false_with d (IncompatibleAsyncSorts (RelArg.context d, t1, t2))

let rec rel_list item_name d p rel eq xs1 xs2 =
  match xs1, xs2 with
  | [], [] -> true
  | [], _ -> fewer_items d item_name
  | _, [] -> more_items d item_name
  | x1::rest1, x2::rest2 ->
    p d rel eq x1 x2 &&
    rel_list item_name d p rel eq rest1 rest2

let rec rel_typ d rel eq t1 t2 =
  let d = RelArg.inc_depth d in
  if RelArg.exceeds_max_depth d then raise Undecided else
  t1 == t2 || SS.mem (t1, t2) !rel || begin
  rel := SS.add (t1, t2) !rel;
  match t1, t2 with
  (* Second-class types first, since they mustn't relate to Any/Non *)
  | Pre, _ | _, Pre ->
    raise PreEncountered
  | Mut t1', Mut t2' ->
    eq_typ d rel eq t1' t2'
  | Typ c1, Typ c2 ->
    eq_con d eq c1 c2 || incompatible_types d t1 t2
  | Mut _, _ | _, Mut _
  | Typ _, _ | _, Typ _ ->
    incompatible_types d t1 t2
  | Any, Any ->
    true
  | _, Any when rel != eq ->
    not (RelArg.is_stable_sub d) || promotion_to_any d t1
  | Non, Non ->
    true
  | Non, _ when rel != eq ->
    true
  | Named (n, t1'), t2 ->
    rel_typ d rel eq t1' t2
  | t1, Named (n, t2') ->
    let d' = RelArg.push (NamedType n) d in
    rel_typ d' rel eq t1 t2'
  | Con (con1, ts1), Con (con2, ts2) ->
    (match Cons.kind con1, Cons.kind con2 with
    | Def (tbs, t), _ -> (* TBR this may fail to terminate *)
      rel_typ d rel eq (open_ ts1 t) t2
    | _, Def (tbs, t) -> (* TBR this may fail to terminate *)
      let d' = RelArg.push (ConsType con2) d in
      rel_typ d' rel eq t1 (open_ ts2 t)
    | _ when Cons.eq con1 con2 ->
      rel_list "type arguments" d eq_typ rel eq ts1 ts2
    | Abs (tbs, t), _ when rel != eq ->
      rel_typ d rel eq (open_ ts1 t) t2
    | _ ->
      incompatible_types d t1 t2
    )
  | Con (con1, ts1), t2 ->
    (match Cons.kind con1, t2 with
    | Def (tbs, t), _ -> (* TBR this may fail to terminate *)
      rel_typ d rel eq (open_ ts1 t) t2
    | Abs (tbs, t), _ when rel != eq ->
      rel_typ d rel eq (open_ ts1 t) t2
    | _ -> incompatible_types d t1 t2
    )
  | t1, Con (con2, ts2) ->
    (match Cons.kind con2 with
     | Def (tbs, t) -> (* TBR this may fail to terminate *)
       let d' = RelArg.push (ConsType con2) d in
       rel_typ d' rel eq t1 (open_ ts2 t)
     | _ -> incompatible_types d t1 t2
    )
  | Prim p1, Prim p2 when p1 = p2 ->
    true
  | Prim p1, Prim p2 when rel != eq ->
    (p1 = Nat && p2 = Int) ||
    incompatible_prims d t1 t2
  | Obj (s1, tfs1), Obj (s2, tfs2) ->
    (s1 = s2 || incompatible_obj_sorts d t1 t2) &&
    rel_fields t2 d rel eq tfs1 tfs2
  | Array t1', Array t2' ->
    rel_typ d rel eq t1' t2'
  | Opt t1', Opt t2' ->
    rel_typ d rel eq t1' t2'
  | Weak t1', Weak t2' ->
    rel_typ d rel eq t1' t2'
  | Prim Null, Opt t2' when rel != eq ->
    true
  | Variant fs1, Variant fs2 ->
    rel_tags t2 d rel eq fs1 fs2
  | Tup ts1, Tup ts2 ->
    rel_list "tuple arguments" d rel_typ rel eq ts1 ts2
  | Func (s1, c1, tbs1, t11, t12), Func (s2, c2, tbs2, t21, t22) ->
    (s1 = s2 || incompatible_func_sorts d t1 t2) &&
    (c1 = c2 || incompatible_bounds d t1 t2) &&
    (match rel_binds d eq eq tbs1 tbs2 with
     | Some ts ->
        rel_list "function parameters" d rel_typ rel eq (List.map (open_ ts) t21) (List.map (open_ ts) t11) &&
        rel_list "return types" d rel_typ rel eq (List.map (open_ ts) t12) (List.map (open_ ts) t22)
     | None -> incompatible_funcs d t1 t2
    )
  | Async (s1, t11, t12), Async (s2, t21, t22) ->
    (s1 = s2 || incompatible_async_sorts d t1 t2) &&
    eq_typ d rel eq t11 t21 &&
    rel_typ d rel eq t12 t22
  | _, _ -> incompatible_types d t1 t2
  end

and rel_fields t2 d rel eq tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  match tfs1, tfs2 with
  | [], [] ->
    true
  | tf1::_, [] when rel != eq && not (RelArg.is_stable_sub d) ->
    true
  | tf1::tfs1', tf2::tfs2' ->
    (match compare_field tf1 tf2 with
    | 0 ->
      let d' = RelArg.push (Field tf2.lab) d in
      let is_rel =
        rel_typ d' rel eq tf1.typ tf2.typ &&
        rel_fields t2 d rel eq tfs1' tfs2'
      in
      add_src_field_update is_rel rel eq tf1 tf2;
      is_rel
    | -1 when rel != eq && not (RelArg.is_stable_sub d) ->
      rel_fields t2 d rel eq tfs1' tfs2
    | result ->
      if result > 0 then
        unexpected_field d tf2.lab t2
      else
        missing_field d tf1.lab t2
    )
  | [], tf2::_ ->
    unexpected_field d tf2.lab t2
  | tf1::_, [] ->
    missing_field d tf1.lab t2

and rel_tags t2 d rel eq tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  match tfs1, tfs2 with
  | [], [] ->
    true
  | [], _ when rel != eq ->
    true
  | tf1::tfs1', tf2::tfs2' ->
    (match compare_field tf1 tf2 with
     | 0 ->
      let is_rel =
       rel_typ d rel eq tf1.typ tf2.typ &&
       rel_tags t2 d rel eq tfs1' tfs2'
      in
      add_src_field_update is_rel rel eq tf1 tf2;
      is_rel
    | +1 when rel != eq ->
      rel_tags t2 d rel eq tfs1 tfs2'
    | result ->
      if result > 0 then
        unexpected_tag d tf2.lab t2
      else
        missing_tag d tf1.lab t2
    )
  | [], tf2::_ ->
    unexpected_tag d tf2.lab t2
  | tf1::_, [] ->
    missing_tag d tf1.lab t2

and rel_binds d rel eq tbs1 tbs2 =
  let ts = open_binds_unbounded tbs2 in
  if rel_list "type parameters" d (rel_bind ts) rel eq tbs2 tbs1
  then Some ts
  else None

and rel_bind ts d rel eq tb1 tb2 =
  tb1.sort == tb2.sort &&
  rel_typ d rel eq (open_ ts tb1.bound) (open_ ts tb2.bound)

and eq_typ d rel eq t1 t2 = rel_typ d eq eq t1 t2

and eq_kind' eq k1 k2 : bool =
  match k1, k2 with
  | Def (tbs1, t1), Def (tbs2, t2)
  | Abs (tbs1, t1), Abs (tbs2, t2) ->
    (match rel_binds (RelArg.sub []) eq eq tbs1 tbs2 with
    | Some ts -> eq_typ (RelArg.sub []) eq eq (open_ ts t1) (open_ ts t2)
    | None -> false
    )
  | _ -> false

and eq_con d eq c1 c2 =
  match Cons.kind c1, Cons.kind c2 with
  | (Def (tbs1, t1)) as k1, (Def (tbs2, t2) as k2) ->
    eq_kind' eq k1 k2
  | Abs _, Abs _ ->
    Cons.eq c1 c2
  | Def (tbs1, t1), Abs (tbs2, t2)
  | Abs (tbs2, t2), Def (tbs1, t1) ->
    (match rel_binds d eq eq tbs1 tbs2 with
    | Some ts -> eq_typ d eq eq (open_ ts t1) (Con (c2, ts))
    | None -> false
    )

let eq_binds ?(src_fields = empty_srcs_tbl ()) tbs1 tbs2 =
  with_src_field_updates_predicate src_fields (fun () ->
    let eq = ref SS.empty in rel_binds (RelArg.sub []) eq eq tbs1 tbs2 <> None)

let eq ?(src_fields = empty_srcs_tbl ()) t1 t2 : bool =
  with_src_field_updates_predicate src_fields (fun () ->
    let eq = ref SS.empty in eq_typ (RelArg.sub []) eq eq t1 t2)

let eq_kind ?(src_fields = empty_srcs_tbl ()) k1 k2 : bool =
  with_src_field_updates_predicate src_fields (fun () ->
    eq_kind' (ref SS.empty) k1 k2)

let sub_explained ?(src_fields = empty_srcs_tbl ()) context t1 t2 =
  with_src_field_updates_predicate_general src_fields (fun () ->
    let d = RelArg.sub context in
    if rel_typ d (ref SS.empty) (ref SS.empty) t1 t2
    then Compatible else match RelArg.explanation d with
      | Some e -> Incompatible e
      | None -> Incompatible (IncompatibleTypes (context, t1, t2)))
    (fun result -> result = Compatible)

let sub ?(src_fields = empty_srcs_tbl ()) t1 t2 : bool =
  with_src_field_updates_predicate src_fields (fun () ->
    rel_typ (RelArg.sub []) (ref SS.empty) (ref SS.empty) t1 t2)

(* Compatibility *)

let compatible_list p co xs1 xs2 =
  try List.for_all2 (p co) xs1 xs2 with Invalid_argument _ -> false

let rec compatible_typ co t1 t2 =
  t1 == t2 || SS.mem (t1, t2) !co || begin
  co := SS.add (t1, t2) !co;
  match promote t1, promote t2 with
  | Pre, _ | _, Pre ->
    assert false
  | Mut t1', Mut t2' ->
    compatible_typ co t1' t2'
  | Typ _, Typ _ ->
    true
  | Mut _, _ | _, Mut _
  | Typ _, _ | _, Typ _ ->
    false
  | Any, Any ->
    true
  | Any, _ | _, Any ->
    false
  | Non, _ | _, Non ->
    true
  | Prim p1, Prim p2 when p1 = p2 ->
    true
  | Prim (Nat | Int), Prim (Nat | Int) ->
    true
  | Array t1', Array t2' ->
    compatible_typ co t1' t2'
  | Tup ts1, Tup ts2 ->
    compatible_list compatible_typ co ts1 ts2
  | Obj (s1, tfs1), Obj (s2, tfs2) ->
    s1 = s2 &&
    compatible_fields co tfs1 tfs2
  | Opt t1', Opt t2' ->
    compatible_typ co t1' t2'
  | Weak t1', Weak t2' ->
    compatible_typ co t1' t2' (* TBR *)
  | Prim Null, Opt _ | Opt _, Prim Null  ->
    true
  | Variant tfs1, Variant tfs2 ->
    compatible_tags co tfs1 tfs2
  | Async (s1, t11, t12), Async (s2, t21, t22) ->
    s1 = s2 &&
    compatible_typ co t11 t21 && (* TBR *)
    compatible_typ co t12 t22
  | Func _, Func _ ->
    true
  | Named _, _
  | _, Named _ -> assert false
  | _, _ ->
    false
  end

and compatible_fields co tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  match tfs1, tfs2 with
  | [], [] -> true
  | tf1::tfs1', tf2::tfs2' ->
    tf1.lab = tf2.lab && compatible_typ co tf1.typ tf2.typ &&
    compatible_fields co tfs1' tfs2'
  | _, _ -> false

and compatible_tags co tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  match tfs1, tfs2 with
  | [], _ | _, [] -> true
  | tf1::tfs1', tf2::tfs2' ->
    match compare_field tf1 tf2 with
    | -1 -> compatible_tags co tfs1' tfs2
    | +1 -> compatible_tags co tfs1 tfs2'
    | _ -> compatible_typ co tf1.typ tf2.typ && compatible_tags co tfs1' tfs2'

and compatible t1 t2 : bool =
  compatible_typ (ref SS.empty) t1 t2


let opaque t = compatible t Any


(* Inhabitance *)

let rec inhabited_typ co t =
  S.mem t !co || begin
  co := S.add t !co;
  match promote t with
  | Pre -> assert false
  | Non -> false
  | Any | Prim _ | Array _ | Opt _ | Async _ | Func _ | Typ _ -> true
  | Weak t' (* TBR *)
  | Mut t' -> inhabited_typ co t'
  | Tup ts -> List.for_all (inhabited_typ co) ts
  | Obj (_, tfs) -> List.for_all (inhabited_field co) tfs
  | Variant tfs -> List.exists (inhabited_field co) tfs
  | Var _ -> true  (* TODO(rossberg): consider bound *)
  | Con (c, ts) ->
    (match Cons.kind c with
    | Def (tbs, t') -> (* TBR this may fail to terminate *)
      inhabited_typ co (open_ ts t')
    | Abs (tbs, t') ->
      inhabited_typ co t')
  | Named _ -> assert false
  end

and inhabited_field co tf = inhabited_typ co tf.typ

and inhabited t : bool = inhabited_typ (ref S.empty) t

let rec singleton_typ co t =
  S.mem t !co || begin
  co := S.add t !co;
  match normalize t with
  | Pre -> assert false
  | Prim Null | Any -> true
  | Tup ts -> List.for_all (singleton_typ co) ts
  | Obj ((Object|Memory|Module), fs) -> List.for_all (singleton_field co) fs
  | Variant [f] -> singleton_field co f

  | Non -> false
  | Prim _ | Array _ | Opt _ | Async _ | Func _ | Typ _ -> false
  | Weak _
  | Mut _ -> false
  | Obj (_, _) -> false
  | Variant _ -> false
  | Var _ -> false
  | Con _ -> false
  | Named _ -> assert false
  end

and singleton_field co tf = singleton_typ co tf.typ

and singleton t : bool = singleton_typ (ref S.empty) t

type no_subtypes_or_supertypes =
  | NoSubtypes
  | NoSupertypes

let rec has_no_subtypes_or_supertypes m co = function
  | Con (c, ts) as t ->
    (match Cons.kind c with
    | Abs (_, bound) ->
      (* Type parameters have no proper subtypes *)
      m = NoSubtypes ||
      (* And no proper supertypes when unbounded. *)
      bound = Any
    | Def (tbs, def) ->
      let s = fst co in
      S.mem t !s || begin
        s := S.add t !s;
        has_no_subtypes_or_supertypes m co (reduce tbs def ts)
      end
    )
  | Mut _ -> true
  | Named (_, t) -> has_no_subtypes_or_supertypes m co t
  | Prim
    ( Bool
    | Nat8
    | Nat16
    | Nat32
    | Nat64
    | Int8
    | Int16
    | Int32
    | Int64
    | Float
    | Char
    | Text
    | Blob
    | Error
    | Principal
    | Region
    ) -> true
  (* Null <: ?T so Null has no proper subtypes and ?T has no proper supertypes. *)
  | Opt t -> m = NoSupertypes && has_no_subtypes_or_supertypes m co t
  (* Nat <: Int so Nat has no proper subtypes and Int has no proper supertypes. *)
  | Prim Int -> m = NoSupertypes
  | Prim (Null | Nat) -> m = NoSubtypes
  | Array t
  | Async (_, _, t)
  | Weak t -> has_no_subtypes_or_supertypes m co t
  | Tup ts -> List.for_all (has_no_subtypes_or_supertypes m co) ts
  | Func (_, _, _, ts1, ts2) ->
    let flip = function
      | NoSubtypes -> NoSupertypes
      | NoSupertypes -> NoSubtypes
    in
    let swap (s1, s2) = (s2, s1) in
    (* For func inputs, flip the mode and swap the result sets. *)
    List.for_all (has_no_subtypes_or_supertypes (flip m) (swap co)) ts1 &&
    List.for_all (has_no_subtypes_or_supertypes       m        co ) ts2
  | _ -> false

and has_no_supertypes t = has_no_subtypes_or_supertypes NoSupertypes (ref S.empty, ref S.empty) t

and has_no_subtypes t = has_no_subtypes_or_supertypes NoSubtypes (ref S.empty, ref S.empty) t

(* Least upper bound and greatest lower bound *)

module M = Map.Make (OrdPair)

exception Mismatch

let rec combine rel lubs glbs t1 t2 =
  assert (rel == lubs || rel == glbs);
  if t1 == t2 then t1 else
  match M.find_opt (t1, t2) !rel with
  | Some t -> t
  | _ when eq t1 t2 ->
    let t = if is_con t2 then t2 else t1 in
    rel := M.add (t2, t1) t (M.add (t1, t2) t !rel);
    t
  | _ ->
    match t1, t2 with
    | Pre, _ | _, Pre ->
      raise PreEncountered
    | Mut _, _ | _, Mut _
    | Typ _, _ | _, Typ _ ->
      raise Mismatch
    | Any, t | t, Any ->
      if rel == lubs then Any else t
    | Non, t | t, Non ->
      if rel == lubs then t else Non
    | Prim Nat, Prim Int
    | Prim Int, Prim Nat ->
      Prim (if rel == lubs then Int else Nat)
    | Opt t1', Opt t2' ->
      Opt (combine rel lubs glbs t1' t2')
    | Weak t1', Weak t2' ->
      Weak (combine rel lubs glbs t1' t2')
    | (Opt _ as t), (Prim Null as t')
    | (Prim Null as t'), (Opt _ as t) ->
      if rel == lubs then t else t'
    | Array t1', Array t2' ->
      (try Array (combine rel lubs glbs t1' t2')
       with Mismatch -> if rel == lubs then Any else Non)
    | Variant t1', Variant t2' ->
      Variant (combine_tags rel lubs glbs t1' t2')
    | Tup ts1, Tup ts2 when List.(length ts1 = length ts2) ->
      Tup (List.map2 (combine rel lubs glbs) ts1 ts2)
    | Obj (s1, tf1), Obj (s2, tf2) when s1 = s2 ->
      (try Obj (s1, combine_fields rel lubs glbs tf1 tf2)
      with Mismatch -> assert (rel == glbs); Non)
    | Func (s1, c1, bs1, ts11, ts12), Func (s2, c2, bs2, ts21, ts22) when
        s1 = s2 && c1 = c2 && eq_binds bs1 bs2 &&
        List.(length ts11 = length ts21 && length ts12 = length ts22) ->
      let ts = open_binds_unbounded bs1 in
      let cs = List.map (fun t -> fst (as_con t)) ts in
      let opened = List.map (open_ ts) in
      let closed = List.map (close cs) in
      let rel' = if rel == lubs then glbs else lubs in
      Func (
        s1, c1, bs1,
        closed (List.map2 (combine rel' lubs glbs) (opened ts11) (opened ts21)),
        closed (List.map2 (combine rel lubs glbs) (opened ts12) (opened ts22))
      )
    | Async (s1, t11, t12), Async (s2, t21, t22) when s1 == s2 && eq t11 t21 ->
      Async (s1, t11, combine rel lubs glbs t12 t22)
    | Con _, _
    | _, Con _ ->
      if sub t1 t2 then
        let t = if rel == glbs then t1 else t2 in
        rel := M.add (t2, t1) t (M.add (t1, t2) t !rel);
        t
      else if sub t2 t1 then
        let t = if rel == lubs then t1 else t2 in
        rel := M.add (t2, t1) t (M.add (t1, t2) t !rel);
        t
      else
        let op, expand =
          if rel == lubs then "lub", promote else "glb", normalize in
        let name = op ^ "<" ^ !str t1 ^ ", " ^ !str t2 ^ ">" in
        let c = Cons.fresh name (Abs ([], Pre)) in
        let t = Con (c, []) in
        rel := M.add (t2, t1) t (M.add (t1, t2) t !rel);
        let t' =
          (* When taking the glb of an abstract con and an incompatible type,
           * normalisation will no further simplify t1 nor t2, so that t itself
           * is returned via the extended relation. In that case, bottom is
           * the correct result.
           *)
          match combine rel lubs glbs (expand t1) (expand t2) with
          | t' when t' == t -> assert (rel == glbs); Non
          | t' -> t'
        in
        set_kind c (Def ([], t'));
        t'
    | Named (n1, t1'), Named (n2, t2') ->
      if n1 = n2 then
        Named (n1, combine rel lubs glbs t1' t2')
      else
        combine rel lubs glbs t1' t2'
    | Named (_, t), t'
    | t, Named (_, t') ->
      combine rel lubs glbs t t'
    | _, _ ->
      if rel == lubs then Any else Non

and cons_if b x xs = if b then x::xs else xs

and combine_fields rel lubs glbs fs1 fs2 =
  match fs1, fs2 with
  | _, [] -> if rel == lubs then [] else fs1
  | [], _ -> if rel == lubs then [] else fs2
  | f1::fs1', f2::fs2' ->
    match compare_field f1 f2 with
    | -1 -> cons_if (rel == glbs) f1 (combine_fields rel lubs glbs fs1' fs2)
    | +1 -> cons_if (rel == glbs) f2 (combine_fields rel lubs glbs fs1 fs2')
    | _ ->
      match combine rel lubs glbs f1.typ f2.typ with
      | typ ->
        add_src_field rel lubs glbs f1 f2;
        let src = {empty_src with track_region = f1.src.track_region} in
        {lab = f1.lab; typ; src} :: combine_fields rel lubs glbs fs1' fs2'
      | exception Mismatch when rel == lubs ->
        combine_fields rel lubs glbs fs1' fs2'

and combine_tags rel lubs glbs fs1 fs2 =
  match fs1, fs2 with
  | _, [] -> if rel == lubs then fs1 else []
  | [], _ -> if rel == lubs then fs2 else []
  | f1::fs1', f2::fs2' ->
    match compare_field f1 f2 with
    | -1 -> cons_if (rel == lubs) f1 (combine_tags rel lubs glbs fs1' fs2)
    | +1 -> cons_if (rel == lubs) f2 (combine_tags rel lubs glbs fs1 fs2')
    | _ ->
      let typ = combine rel lubs glbs f1.typ f2.typ in
      add_src_field rel lubs glbs f1 f2;
      let src = {empty_src with track_region = f1.src.track_region} in
      {lab = f1.lab; typ; src} :: combine_tags rel lubs glbs fs1' fs2'

let lub ?(src_fields = empty_srcs_tbl ()) t1 t2 =
  with_src_field_updates src_fields (fun () ->
    let lubs = ref M.empty in
    combine lubs lubs (ref M.empty) t1 t2)

let glb ?(src_fields = empty_srcs_tbl ()) t1 t2 =
  with_src_field_updates src_fields (fun () ->
    let glbs = ref M.empty in
    combine glbs (ref M.empty) glbs t1 t2)


(* Environments *)

module Env = Env.Make(String)

(* Scopes *)

let scope_var var = "$" ^ var
let default_scope_var = scope_var ""
let scope_bound = Any
let scope_bind = { var = default_scope_var; sort = Scope; bound = scope_bound }

(* Shorthands for replica callbacks *)

let heartbeat_type =
  Func (Local, Returns, [scope_bind], [], [Async (Fut, Var (default_scope_var, 0), unit)])

let global_timer_set_type = Func (Local, Returns, [], [Prim Nat64], [])

let timer_type =
  Func (Local, Returns, [scope_bind],
        [global_timer_set_type],
        [Async (Fut, Var (default_scope_var, 0), unit)])

let low_memory_type =
  Func (Local, Returns, [scope_bind], [], [Async (Cmp, Var (default_scope_var, 0), unit)])

(* Well-known fields *)

let motoko_async_helper_fld =
  { lab = "__motoko_async_helper";
    typ = Func(Shared Write, Promises, [scope_bind], [Prim Nat32], []);
    src = empty_src;
  }

let motoko_stable_var_info_fld =
  { lab = "__motoko_stable_var_info";
    typ =
      Func(Shared Query, Promises, [scope_bind], [],
        [ Obj(Object, [{lab = "size"; typ = nat64; src = empty_src}]) ]);
    src = empty_src;
  }

let motoko_gc_trigger_fld =
  { lab = "__motoko_gc_trigger";
    typ = Func(Shared Write, Promises, [scope_bind], [], []);
    src = empty_src;
  }

let motoko_runtime_information_type =
  Obj(Object, [
    (* Fields must be sorted by label *)
    {lab = "callbackTableCount"; typ = nat; src = empty_src};
    {lab = "callbackTableSize"; typ = nat; src = empty_src};
    {lab = "compilerVersion"; typ = text; src = empty_src};
    {lab = "garbageCollector"; typ = text; src = empty_src};
    {lab = "heapSize"; typ = nat; src = empty_src};
    {lab = "logicalStableMemorySize"; typ = nat; src = empty_src};
    {lab = "maxLiveSize"; typ = nat; src = empty_src};
    {lab = "maxStackSize"; typ = nat; src = empty_src};
    {lab = "memorySize"; typ = nat; src = empty_src};
    {lab = "reclaimed"; typ = nat; src = empty_src};
    {lab = "rtsVersion"; typ = text; src = empty_src};
    {lab = "sanityChecks"; typ = bool; src = empty_src};
    {lab = "stableMemorySize"; typ = nat; src = empty_src};
    {lab = "totalAllocation"; typ = nat; src = empty_src};
  ])

let motoko_runtime_information_fld =
  { lab = "__motoko_runtime_information";
    typ = Func(Shared Query, Promises, [scope_bind], [],
      [ motoko_runtime_information_type ]);
    src = empty_src;
  }

let well_known_actor_fields = [
    motoko_async_helper_fld;
    motoko_stable_var_info_fld;
    motoko_gc_trigger_fld;
  ]

let decode_msg_typ tfs =
  Variant
    (List.sort compare_field (List.filter_map (fun tf ->
       match normalize tf.typ with
       | Func(Shared (Write | Query), _, tbs, ts1, ts2) ->
         Some { tf with
           typ =
             Func(Local, Returns, [], [],
               List.map (open_ (List.map (fun _ -> Non) tbs)) ts1);
           src = empty_src }
       | _ -> None)
     tfs))

let canister_settings_typ =
  obj Object [
    "settings",
    Opt (
      obj Object [
      ("controllers", Opt (Array principal));
      ("compute_allocation", Opt nat);
      ("memory_allocation", Opt nat);
      ("freezing_threshold", Opt nat)])]

let wasm_memory_persistence_typ =
  sum [
    ("keep", unit);
    ("replace", unit);
  ]

let upgrade_with_persistence_option_typ =
  obj Object [
    ("wasm_memory_persistence", wasm_memory_persistence_typ);
    ("canister", obj Actor []);
  ]

let install_arg_typ =
  sum [
    ("new", canister_settings_typ);
    ("install", principal);
    ("reinstall", obj Actor []);
    ("upgrade", obj Actor []);
    ("upgrade_with_persistence", upgrade_with_persistence_option_typ );
  ]

let install_typ ts actor_typ =
  Func(Local, Returns, [],
    [ install_arg_typ ],
    [ Func(Local, Returns, [scope_bind], ts, [Async (Fut, Var (default_scope_var, 0), actor_typ)]) ])

let cycles_lab = "cycles"
let migration_lab = "migration"
let timeout_lab = "timeout"

let cycles_fld = { lab = cycles_lab; typ = nat; src = empty_src }
let timeout_fld = { lab = timeout_lab; typ = nat32; src = empty_src }

(* Pretty printing *)

let string_of_async_sort = function
  | Fut -> ""
  | Cmp -> "*"

let string_of_prim = function
  | Null -> "Null"
  | Bool -> "Bool"
  | Nat -> "Nat"
  | Nat8 -> "Nat8"
  | Nat16 -> "Nat16"
  | Nat32 -> "Nat32"
  | Nat64 -> "Nat64"
  | Int -> "Int"
  | Int8 -> "Int8"
  | Int16 -> "Int16"
  | Int32 -> "Int32"
  | Int64 -> "Int64"
  | Float -> "Float"
  | Char -> "Char"
  | Text -> "Text"
  | Blob -> "Blob"
  | Error -> "Error"
  | Principal -> "Principal"
  | Region -> "Region"

let string_of_obj_sort = function
  | Object -> ""
  | Module -> "module "
  | Actor -> "actor "
  | Mixin -> "mixin "
  | Memory -> "memory "

let string_of_func_sort = function
  | Local -> ""
  | Shared Write -> "shared "
  | Shared Query -> "shared query "
  | Shared Composite -> "shared composite query " (* TBR *)

(* PrettyPrinter configurations *)

module type PrettyConfig = sig
  val show_stamps : bool
  val show_scopes : bool
  val show_hash_suffix : bool (* TODO: remove once we pretty print stable sigs without hashes *)
  val con_sep : string
  val par_sep : string
  val max_list : int option
end

module ShowStamps = struct
  let show_stamps = true
  let show_scopes = true
  let show_hash_suffix = true
  let con_sep = "__" (* TODO: revert to "/" *)
  let par_sep = "_"
  let max_list = None
end

module ParseableStamps = struct
  let show_stamps = true
  let show_scopes = true (* false ok too *)
  let con_sep = "__"
  let par_sep = "_"
  let show_hash_suffix = true
  let max_list = None
end

module ElideStamps = struct
  let show_stamps = false
  let show_scopes = true
  let show_hash_suffix = true
  let con_sep = ShowStamps.con_sep
  let par_sep = ShowStamps.par_sep
  let max_list = None
end

module ElideStampsAndHashes = struct
  include ElideStamps
  let show_hash_suffix = false
  let max_list = Some 10
end

module MakePretty(Cfg : PrettyConfig) = struct

let remove_hash_suffix s =
  let len = String.length s in
  if len = 0 then s
  else
    (* Find the end of any trailing digits *)
    let rec find_digit_end i =
      if i < 0 then -1
      else if s.[i] >= '0' && s.[i] <= '9' then find_digit_end (i - 1)
      else i
    in
    let digit_end = find_digit_end (len - 1) in
    (* Check if we found digits and they're preceded by "__" *)
    if digit_end >= 0 && digit_end < len - 1 && digit_end >= 1 &&
       s.[digit_end] = '_' && s.[digit_end - 1] = '_' then
      String.sub s 0 (digit_end - 1)
    else
      s


open Format

(* To format a list *)
let pp_print_list ?(pp_sep = pp_print_cut) pp_v ppf v =
  match Cfg.max_list with
  | None -> pp_print_list ~pp_sep pp_v ppf v
  | Some max ->
     let len = List.length v in
     if len < max then
       pp_print_list ~pp_sep pp_v ppf v
     else
       let pre_vs = Lib.List.take (max / 2) v in
       let post_vs = Lib.List.drop (len - (max / 2)) v in
       pp_print_list ~pp_sep pp_v ppf pre_vs;
       pp_sep ppf ();
       fprintf ppf "...@ ";
       pp_print_list ~pp_sep pp_v ppf post_vs

let pr = pp_print_string

let comma ppf () = fprintf ppf ",@ "

let semi ppf () = fprintf ppf ";@ "

module StringSet = Set.Make(String)

let vs_of_cs cs =
  let names = ConSet.fold (fun c ns -> StringSet.add (Cons.name c) ns) cs StringSet.empty in
  StringSet.fold (fun n vs -> (n, 0)::vs) names []

let string_of_var (x, i) =
  if i = 0 then sprintf "%s" x else sprintf "%s%s%d" x Cfg.par_sep i

let string_of_con c =
  let name = Cons.to_string Cfg.show_stamps Cfg.con_sep c in
  if Cfg.show_hash_suffix then name
  else remove_hash_suffix name

let rec can_sugar = function
  | Func(s, Promises, tbs, ts1, ts2)
  | Func((Shared _ as s), Returns, tbs, ts1, ([] as ts2))
  | Func(s, Returns, (_::_ as tbs), ts1, ([Async (_, Var(_, 0),_)] as ts2)) ->
    List.for_all (fun tb -> can_omit 0 tb.bound) tbs &&
    List.for_all (can_omit 0) ts1 &&
    List.for_all (can_omit 0) ts2
  | _ -> false

and can_omit n t =
  let rec go i = function
    | Var (_, j) -> i <> j
    | Pre -> assert false
    | Prim _ | Any | Non -> true
    | Con (c, ts) -> List.for_all (go i ) ts
    | Array t | Opt t | Mut t | Weak t -> go i t
    | Async (s, Var (_, j), t2) when j = i && i <= n -> go i t2 (* t1 is a phantom type *)
    | Async (s, t1, t2) -> go i t1 && go i t2
    | Tup ts -> List.for_all (go i ) ts
    | Obj (_, fs) | Variant fs -> List.for_all (fun f -> go i f.typ) fs
    | Func (s, c, tbs, ts1, ts2) ->
      let i' = i+List.length tbs in
      List.for_all (fun tb -> go i' tb.bound) tbs &&
      List.for_all (go i') ts1 &&
      List.for_all (go i') ts2
    | Typ c -> true (* assumes type defs are closed *)
    | Named (n, t) -> go i t
  in go n t

let rec pp_typ_obj vs ppf o =
  match o with
  | (Object, fs) ->
    fprintf ppf "@[<hv 2>{@;<0 0>%a@;<0 -2>}@]"
      (pp_print_list ~pp_sep:semi (pp_field vs)) fs
  | (s, fs) ->
    fprintf ppf "@[<hv 2>%s{@;<0 0>%a@;<0 -2>}@]"
      (string_of_obj_sort s)
      (pp_print_list ~pp_sep:semi (pp_field vs)) fs

and pp_typ_variant vs ppf fs =
  match fs with
  | [] -> pr ppf "{#}"
  | fs ->
    fprintf ppf "@[<hv 2>{@;<0 0>%a@;<0 -2>}@]"
      (pp_print_list ~pp_sep:semi (pp_tag vs)) fs

and pp_typ_item vs ppf t =
  match t with
  | Named (n, t) ->
    fprintf ppf "@[<1>%s : %a@]" n (pp_typ' vs) t
  | typ -> pp_typ' vs ppf t

and pp_typ_nullary vs ppf t =
  match t with
  | Named (n, t) ->
    fprintf ppf "@[<1>(%s : %a)@]" n (pp_typ' vs) t
  | Tup ts ->
    fprintf ppf "@[<1>(%a%s)@]"
      (pp_print_list ~pp_sep:comma (pp_typ_item vs)) ts
      (if List.length ts = 1 then "," else "")
  | Pre -> pr ppf "???"
  | Any -> pr ppf "Any"
  | Non -> pr ppf "None"
  | Prim p -> pr ppf (string_of_prim p)
  | Var (s, i) ->
    pr ppf (try string_of_var (List.nth vs i) with _ -> Printf.sprintf "??? %s %i" s i)
  | Con (c, []) -> pr ppf (string_of_con c)
  | Con (c, ts) ->
    fprintf ppf "@[%s<@[<1>%a@]>@]" (string_of_con c)
      (pp_print_list ~pp_sep:comma (pp_typ' vs)) ts
  | Array (Mut t) ->
    fprintf ppf "@[<1>[var %a]@]" (pp_typ' vs) t
  | Array t ->
    fprintf ppf "@[<1>[%a]@]" (pp_typ' vs) t
  | Obj (Object, fs) ->
    pp_typ_obj vs ppf (Object, fs)
  | Variant fs ->
    pp_typ_variant vs ppf fs
  | t ->
    (* In the parser, this case is subsumed by the grammar production for `LPAR .. RPAR` *)
    fprintf ppf "@[<1>(%a)@]" (pp_typ' vs) t

and pp_typ_un vs ppf t =
  match t with
  | Opt t ->
    fprintf ppf "@[<1>?%a@]"  (pp_typ_un vs) t
  | Weak t ->
    fprintf ppf "@[<1>weak %a@]"  (pp_typ_un vs) t
  | t ->
    pp_typ_nullary vs ppf t

and pp_typ_pre vs ppf t =
  match t with
  (* No case for grammar production `PRIM s` *)
  | Async (s, t1, t2) ->
    if Cfg.show_scopes then
      match t1 with
      | Var(_, n) when fst (List.nth vs n) = "" ->
        fprintf ppf "@[<2>async%s@ %a@]" (string_of_async_sort s) (pp_typ_pre vs) t2
      | _ ->
        fprintf ppf "@[<2>async%s<%a>@ %a@]"
          (string_of_async_sort s)
          (pp_typ' vs) t1
          (pp_typ_pre vs) t2
    else fprintf ppf "@[<2>async%s@ %a@]" (string_of_async_sort s) (pp_typ_pre vs) t2
  | Obj ((Module | Actor | Mixin | Memory) as os, fs) ->
    pp_typ_obj vs ppf (os, fs)
  | t ->
    pp_typ_un vs ppf t

and sequence pp ppf ts =
  match ts with
  | [Tup _] ->
    fprintf ppf "@[<1>(%a)@]" pp (seq ts)
  | ts ->
    pp ppf (seq ts)

and pp_typ_nobin vs ppf t =
  match t with
  | Func (s, c, tbs, ts1, ts2) ->
    let sugar = can_sugar t in
    let vs' = vars_of_binds vs tbs in
    let vs'', tbs' =
      if sugar then
        List.tl vs', List.tl tbs
      else
        match tbs with
        | { sort = Scope; _ } :: _ -> ("system", List.hd vs' |> snd) :: List.tl vs', tbs
        | _ -> vs', tbs
    in
    let vs'vs = vs' @ vs in
    fprintf ppf "@[<2>%s%a%a ->@ %a@]"
      (string_of_func_sort s)
      (pp_binds vs'vs vs'') tbs'
      (sequence (pp_typ_un vs'vs)) ts1
      (pp_control_cod sugar c vs'vs) ts2
  | t ->
     pp_typ_pre vs ppf t

and pp_control_cod sugar c vs ppf ts =
  match c, ts with
  (* sugar *)
  | Returns, [Async (s, _, t)] when sugar ->
    fprintf ppf "@[<2>async%s@ %a@]" (string_of_async_sort s) (pp_typ_pre vs) t
  | Promises, ts ->
    fprintf ppf "@[<2>async@ %a@]" (sequence (pp_typ_pre vs)) ts
  | Returns, _ ->
    sequence (pp_typ_nobin vs) ppf ts
  | Replies, _ ->
    fprintf ppf "@[<2>replies@ %a@]" (sequence (pp_typ_nobin vs)) ts

and pp_typ' vs ppf t =
  match t with
  (* special, additional cases for printing second-class types *)
  | Typ c ->
    fprintf ppf "@[<1>=@ @[(type@ %a)@]@]" (pp_kind' vs) (Cons.kind c)
  | Mut t ->
    fprintf ppf "@[<1>var@ %a@]" (pp_typ_un vs) t
  (* No cases for syntactic _ And _ & _ Or _ (already desugared) *)
  | t -> pp_typ_nobin vs ppf t

and pp_field vs ppf {lab; typ; src} =
  match typ with
  | Typ c ->
    let op, sbs, st = pps_of_kind' vs (Cons.kind c) in
    fprintf ppf "@[<2>type %s%a %s@ %a@]" lab sbs () op st ()
  | Mut t' ->
    fprintf ppf "@[<2>var %s :@ %a@]" lab (pp_typ' vs) t'
  | _ ->
    fprintf ppf "@[<2>%s :@ %a@]" lab (pp_typ' vs) typ

and pp_stab_field vs ppf {lab; typ; src} =
  match typ with
  | Mut t' ->
    fprintf ppf "@[<2>stable var %s :@ %a@]" lab (pp_typ' vs) t'
  | _ ->
    fprintf ppf "@[<2>stable %s :@ %a@]" lab (pp_typ' vs) typ

and pp_pre_stab_field vs ppf (required, {lab; typ; src}) =
  let req = if required then "in" else "stable" in
  match typ with
  | Mut t' ->
    fprintf ppf "@[<2>%s var %s :@ %a@]" req lab (pp_typ' vs) t'
  | _ ->
    fprintf ppf "@[<2>%s %s :@ %a@]" req lab (pp_typ' vs) typ


and pp_tag vs ppf {lab; typ; src} =
  match typ with
  | Tup [] ->
    fprintf ppf "#%s" lab
  | _ ->
    fprintf ppf "@[<2>#%s :@ %a@]" lab (pp_typ' vs) typ

and vars_of_binds vs bs =
  List.map (fun b -> name_of_var vs (b.var, 0)) bs

and name_of_var vs v =
  match vs with
  | [] -> v
  | v'::vs' -> name_of_var vs' (if fst v = fst v' then (fst v, snd v + 1) else v)

and pp_bind vs ppf (v, {bound; _}) =
  if bound = Any then
    pr ppf (string_of_var v)
  else
    fprintf ppf "%s <: %a"
      (string_of_var v)
      (pp_typ' vs) bound

and pp_binds vs vs' ppf = function
  | [] -> ()
  | tbs ->
    fprintf ppf "@[<1><%a>@]"
      (pp_print_list ~pp_sep:comma (pp_bind vs)) (List.combine vs' tbs)

and pps_of_kind' vs k =
  let op, tbs, t =
    match k with
    | Def (tbs, t) -> "=", tbs, t
    | Abs (tbs, t) -> "<:", tbs, t
  in
  let vs' = vars_of_binds vs tbs in
  let vs'vs = vs'@vs in
  op,
  (fun ppf () -> pp_binds vs'vs vs' ppf tbs),
  (fun ppf () -> pp_typ' vs'vs ppf t)

and pps_of_kind k =
  let cs = cons_kind k in
  let vs = vs_of_cs cs in
  pps_of_kind' vs k

and pp_kind' vs ppf k =
  let op, sbs, st = pps_of_kind' vs k in
  fprintf ppf "%s %a%a" op sbs () st ()

and pp_kind ppf k =
  let cs = cons_kind k in
  let vs = vs_of_cs cs in
  pp_kind' vs ppf k

and pp_stab_sig ppf sig_ =
  let all_fields = match sig_ with
    | Single tfs -> tfs
    | PrePost (pre, post) -> List.map snd pre @ post
  in
  let cs = List.fold_right
    (cons_field false)
    (* false here ^ means ignore unreferenced Typ c components
       that would produce unreferenced bindings when unfolded *)
    all_fields ConSet.empty in
  let vs = vs_of_cs cs in
  let ds =
    let cs' = ConSet.filter (fun c ->
      match Cons.kind c with
      | Def ([], Prim p) when string_of_con c = string_of_prim p -> false
      | Def ([], Any) when string_of_con c = "Any" -> false
      | Def ([], Non) when string_of_con c = "None" -> false
      | Def _ -> true
      | Abs _ -> false) cs in
    ConSet.elements cs' in
  let fs =
    List.sort compare_field
      (List.map (fun c ->
        { lab = string_of_con c;
          typ = Typ c;
          src = empty_src }) ds)
  in
  let pp_stab_actor ppf sig_ =
    match sig_ with
    | Single tfs ->
      fprintf ppf "@[<v 2>%s{@;<0 0>%a@;<0 -2>}@]"
        (string_of_obj_sort Actor)
        (pp_print_list ~pp_sep:semi (pp_stab_field vs)) tfs
    | PrePost (pre, post) ->
      fprintf ppf "@[<v 2>%s({@;<0 0>%a@;<0 -2>}, {@;<0 0>%a@;<0 -2>}) @]"
        (string_of_obj_sort Actor)
        (pp_print_list ~pp_sep:semi (pp_pre_stab_field vs)) pre
        (pp_print_list ~pp_sep:semi (pp_stab_field vs)) post
  in
  fprintf ppf "@[<v 0>%a%a%a;@]"
    (pp_print_list ~pp_sep:semi (pp_field vs)) fs
    (if fs = [] then fun ppf () -> () else semi) ()
    pp_stab_actor sig_

let rec pp_typ_expand' vs ppf t =
  match t with
  | Con (c, ts) ->
    (match Cons.kind c with
    | Abs _ -> pp_typ' vs ppf t
    | Def _ ->
      match normalize t with
      | Prim _ | Any | Non -> pp_typ' vs ppf t
      | t' -> fprintf ppf "%a = %a"
        (pp_typ' vs) t
        (pp_typ_expand' vs) t'
    )
  | _ -> pp_typ' vs ppf t

let pp_lab = pr

let pp_typ ppf t =
  let vs = vs_of_cs (cons t) in
  pp_typ' vs ppf t

let pp_typ_expand ppf t =
  let vs = vs_of_cs (cons t) in
  pp_typ_expand' vs ppf t

let string_of_typ typ : string =
  Lib.Format.with_str_formatter (fun ppf ->
    pp_typ ppf) typ

let string_of_kind k : string =
  Lib.Format.with_str_formatter (fun ppf ->
    pp_kind ppf) k

let strings_of_kind k : string * string * string =
  let op, sbs, st = pps_of_kind k in
  op, Lib.Format.with_str_formatter sbs (), Lib.Format.with_str_formatter st ()

let string_of_typ_expand typ : string =
  Lib.Format.with_str_formatter (fun ppf ->
      pp_typ_expand ppf) typ

let string_of_context context =
  let rec emit_context nested context =
    match context with
    | [] -> "top level"
    | (Field label)::rest -> Printf.sprintf "%s.%s" (emit_context nested rest) label
    | (ConsType c)::rest when not nested ->
       Printf.sprintf "%s (used by %s)" (remove_hash_suffix (Cons.name c)) (emit_context true rest)
    | (ConsType c)::rest ->
       Printf.sprintf "%s in %s" (remove_hash_suffix (Cons.name c)) (emit_context true rest)
    | (NamedType name)::rest when not nested ->
       Printf.sprintf "%s (used by %s)" name (emit_context true rest)
    | (NamedType name)::rest ->
       Printf.sprintf "%s in %s" name (emit_context true rest)
    | (StableVariable name)::_ -> name
  in
    emit_context false context

let string_of_explanation explanation =
  let display_typ = Lib.Format.display pp_typ in
  match explanation with
  | IncompatibleTypes (context, t1, t2) ->
    Format.asprintf "The type %a\n is not compatible with type %a\n of %s" display_typ t1 display_typ t2 (string_of_context context)
  | MissingTag (context, lab, t) ->
    Format.asprintf "Missing tag `#%s` in type %a\n of %s" lab display_typ t (string_of_context context)
  | UnexpectedTag (context, lab, t) ->
    Format.asprintf "Unsupported additional tag `#%s` in type %a\n of %s" lab display_typ t (string_of_context context)
  | MissingField (context, lab, t) ->
    Format.asprintf "Missing field `%s` in type %a\n of %s" lab display_typ t (string_of_context context)
  | UnexpectedField (context, lab, t) ->
    Format.asprintf "Unsupported additional field `%s` in type %a\n of %s" lab display_typ t (string_of_context context)
  | FewerItems (context, desc) ->
    Format.asprintf "Fewer %s in %s than expected" desc (string_of_context context)
  | MoreItems (context, desc) ->
    Format.asprintf "More %s in %s than expected" desc (string_of_context context)
  | PromotionToAny (context, t) ->
    Format.asprintf "Converting %a\n to `Any` is disallowed as it leads to data loss: %s" display_typ t (string_of_context context)
  | IncompatiblePrims (context, t1, t2) ->
    let context = match context with h::tl -> tl | _ -> context in
    Format.asprintf "The type %a\n is not compatible with type %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)
  | IncompatibleObjSorts (context, t1, t2) ->
    Format.asprintf "Incompatible object sorts: %a\n does not match %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)
  | IncompatibleFuncSorts (context, t1, t2) ->
    Format.asprintf "Incompatible function modifiers: %a\n does not match %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)
  | IncompatibleBounds (context, t1, t2) ->
    Format.asprintf "Incompatible type parameters: %a\n does not match %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)
  | IncompatibleFuncs (context, t1, t2) ->
    Format.asprintf "Incompatible function types: %a\n does not match %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)
  | IncompatibleAsyncSorts (context, t1, t2) ->
    Format.asprintf "Incompatible async sorts: %a\n does not match %a\n in %s" display_typ t1 display_typ t2 (string_of_context context)

end

module type Pretty = sig
  val pp_lab : Format.formatter -> lab -> unit
  val pp_typ : Format.formatter -> typ -> unit
  val pp_typ_expand : Format.formatter -> typ -> unit
  val pps_of_kind : kind ->
    string *
    (Format.formatter -> unit -> unit) *
      (Format.formatter -> unit -> unit)

  val string_of_con : con -> string
  val string_of_typ : typ -> string
  val string_of_kind : kind -> string
  val strings_of_kind : kind -> string * string * string
  val string_of_typ_expand : typ -> string
  val string_of_explanation : explanation -> string
end

include MakePretty(ElideStamps)

let _ = str := string_of_typ

(* Stable signatures *)
let stable_sub_explained ?(src_fields = empty_srcs_tbl ()) context t1 t2 =
  with_src_field_updates_predicate_general src_fields (fun () ->
    let d = RelArg.stable_sub context in
    if rel_typ d (ref SS.empty) (ref SS.empty) t1 t2
    then Compatible
    else match RelArg.explanation d with
      | Some e -> Incompatible e
      | None -> Incompatible (IncompatibleTypes (context, t1, t2)))
    (fun result -> result = Compatible)

let stable_sub ?(src_fields = empty_srcs_tbl ()) t1 t2 =
  with_src_field_updates_predicate src_fields (fun () ->
    rel_typ (RelArg.stable_sub []) (ref SS.empty) (ref SS.empty) t1 t2)

let pre = function
  | Single tfs ->
    (* all vars optional *)
    List.map (fun tf -> (false, tf)) tfs
  | PrePost (tfs, _) -> tfs

let post = function
  | Single tfs -> tfs
  | PrePost (_, tfs) -> tfs

let rec match_stab_sig sig1 sig2 =
  let post_tfs1 = post sig1 in
  let pre_tfs2 = pre sig2 in
  match_stab_fields post_tfs1 pre_tfs2

and match_stab_fields tfs1 tfs2 =
  (* Assume that tfs1 and tfs2 are sorted. *)
  match tfs1, tfs2 with
  | [], _ ->
    (* same amount of fields or new, non-required, fields ok *)
    List.for_all (fun (required, tf) -> not required) tfs2
  | _, [] ->
    (* no dropped fields *)
    false
  | tf1::tfs1', (required, tf2)::tfs2' ->
    (match compare_field tf1 tf2 with
     | 0 ->
       stable_sub (as_immut tf1.typ) (as_immut tf2.typ) &&
       match_stab_fields tfs1' tfs2'
     | -1 ->
       (* no dropped fields *)
       false
     | _ ->
       (* new field ok *)
       (not required) &&
       match_stab_fields tfs1 tfs2'
    )

let string_of_stab_sig stab_sig : string =
  let module Pretty = MakePretty(ParseableStamps) in
  (match stab_sig with
  | Single _ -> "// Version: 1.0.0\n"
  | PrePost _ -> "// Version: 3.0.0\n") ^
  Format.asprintf "@[<v 0>%a@]@\n" (fun ppf -> Pretty.pp_stab_sig ppf) stab_sig
