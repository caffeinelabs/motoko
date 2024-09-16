open Mo_types
open Mo_values

type id = string

(* Literals *)

type lit =
  | NullLit
  | BoolLit of bool
  | NatLit of Numerics.Nat.t
  | Nat8Lit of Numerics.Nat8.t
  | Nat16Lit of Numerics.Nat16.t
  | Nat32Lit of Numerics.Nat32.t
  | Nat64Lit of Numerics.Nat64.t
  | IntLit of Numerics.Int.t
  | Int8Lit of Numerics.Int_8.t
  | Int16Lit of Numerics.Int_16.t
  | Int32Lit of Numerics.Int_32.t
  | Int64Lit of Numerics.Int_64.t
  | FloatLit of Numerics.Float.t
  | CharLit of Value.unicode
  | TextLit of string
  | BlobLit of string

(* Patterns *)

type 'a phrase = ('a, Note.t) Source.annotated_phrase

type typ_bind' = {con : Type.con; sort : Type.bind_sort; bound : Type.typ}
type typ_bind = typ_bind' Source.phrase

type unop = Operator.unop
type binop = Operator.binop
type relop = Operator.relop

type mut = Const | Var

type pat = (pat', Type.typ) Source.annotated_phrase
and pat' =
  | WildP                                      (* wildcard *)
  | VarP of id                                 (* variable *)
  | LitP of lit                                (* literal *)
  | TupP of pat list                           (* tuple *)
  | ObjP of pat_field list                     (* object *)
  | OptP of pat                                (* option *)
  | TagP of Type.lab * pat                     (* variant *)
  | AltP of pat * pat                          (* disjunctive *)

and pat_field = pat_field' Source.phrase
and pat_field' = {name : Type.lab; pat : pat}

(* Like id, but with a type attached *)
type arg = (string, Type.typ) Source.annotated_phrase

(* Expressions *)

type exp = exp' phrase

and exp' =
  | PrimE of (prim * exp list)                 (* primitive *)
  | VarE of mut * id                           (* variable *)
  | LitE of lit                                (* literal *)
  | AssignE of lexp * exp                      (* assignment *)
  | BlockE of (dec list * exp)                 (* block *)
  | IfE of exp * exp * exp                     (* conditional *)
  | SwitchE of exp * case list                 (* switch *)
  | LoopE of exp                               (* do-while loop *)
  | LabelE of id * Type.typ * exp              (* label *)
  | AsyncE of Type.async_sort * typ_bind * exp * Type.typ        (* async/async* *)
  | DeclareE of id * Type.typ * exp            (* local promise *)
  | DefineE of id * mut * exp                  (* promise fulfillment *)
  | FuncE of                                   (* function *)
      string * Type.func_sort * Type.control * typ_bind list * arg list * Type.typ list * exp
  | SelfCallE of Type.typ list * exp * exp * exp * exp (* essentially ICCallPrim (FuncE shared…) *)
  | ActorE of dec list * field list * system * Type.typ (* actor *)
  | NewObjE of Type.obj_sort * field list * Type.typ     (* make an object *)
  | TryE of exp * case list * (id * Type.typ) option (* try/catch/cleanup *)

and system = {
  meta : meta;
  (* TODO: use option expressions for (some or all of) these *)
  preupgrade : exp;
  postupgrade : exp;
  heartbeat : exp;
  timer : exp; (* TODO: use an option type: (Default of exp | UserDefined of exp) option *)
  inspect : exp;
  stable_record: exp;
  stable_type: Type.typ;
}

and candid = {
    args : string;
    service : string;
  }

and meta = {
    candid : candid;  (* Candid (of service, never actor class) *)
    sig_ : string  (* Motoko stable signature *)
  }

and field = (field', Type.typ) Source.annotated_phrase
and field' = {name : Type.lab; var : id} (* the var is by reference, not by value *)

and case = case' Source.phrase
and case' = {pat : pat; exp : exp}

and lexp = (lexp', Type.typ) Source.annotated_phrase
and lexp' =
  | VarLE of id                                (* variable *)
  | IdxLE of exp * exp                         (* array indexing *)
  | DotLE of exp * Type.lab                    (* object projection *)


(* In the IR, a prim is any AST node that has expr subexpressions, but they are
all call-by-value. Many passes can treat them uniformly, so they are unified
using the PrimE node. *)
and prim =
  | CallPrim of Type.typ list         (* function call *)
  | UnPrim of Type.typ * unop         (* unary operator *)
  | BinPrim of Type.typ * binop       (* binary operator *)
  | RelPrim of Type.typ * relop       (* relational operator *)
  | TupPrim                           (* the tuple constructor *)
  | ProjPrim of int                   (* tuple projection *)
  | OptPrim                           (* option injection *)
  | TagPrim of id                     (* variant injection *)
  | DotPrim of Type.lab               (* object projection *)
  | ActorDotPrim of Type.lab          (* actor field access *)
  | ArrayPrim of mut * Type.typ       (* array constructor *)
  | IdxPrim                           (* array indexing *)
  | BreakPrim of id                   (* break *)
  | RetPrim                           (* return *)
  | AwaitPrim of Type.async_sort       (* await/await* *)
  | AssertPrim                        (* assertion *)
  | ThrowPrim                         (* throw *)
  | ShowPrim of Type.typ              (* debug_show *)
  | SerializePrim of Type.typ list    (* Candid serialization prim *)
  | DeserializePrim of Type.typ list  (* Candid deserialization prim *)
  | DeserializeOptPrim of Type.typ list
     (* Candid deserialization prim (returning Opt) *)
  | NumConvTrapPrim of Type.prim * Type.prim
  | NumConvWrapPrim of Type.prim * Type.prim
  | DecodeUtf8
  | EncodeUtf8
  | CastPrim of Type.typ * Type.typ   (* representationally a noop *)
  | ActorOfIdBlob of Type.typ
  | BlobOfIcUrl                       (* traps on syntax or checksum failure *)
  | IcUrlOfBlob
  | SelfRef of Type.typ               (* returns the self actor ref *)
  | SystemTimePrim
  (* Array field iteration/access *)
  | NextArrayOffset                   (* advance compact array offset, as Nat *)
  | EqArrayOffset                     (* equate compact array offset at type Int *)
  | DerefArrayOffset                  (* compact array offset indexing (unchecked) *)
  | GetLastArrayOffset                (* compact array offset of the last element, or -1, as Int *)
  (* Funds *)
  | SystemCyclesAddPrim
  | SystemCyclesAcceptPrim
  | SystemCyclesAvailablePrim
  | SystemCyclesBalancePrim
  | SystemCyclesRefundedPrim
  | SystemCyclesBurnPrim
  | SetCertifiedData
  | GetCertificate

  | OtherPrim of string               (* Other primitive operation, no custom typing rule *)
  (* backend stuff *)
  | CPSAwait of Type.async_sort * Type.typ
                                      (* typ is the current continuation type of cps translation *)
  | CPSAsync of Type.async_sort * Type.typ
  | ICPerformGC
  | ICReplyPrim of Type.typ list
  | ICRejectPrim
  | ICCallerPrim
  | ICCallPrim
  | ICCallRawPrim
  | ICMethodNamePrim
  | ICArgDataPrim
  | ICStableWrite of Type.typ          (* serialize value of stable type to stable memory *)
  | ICStableRead of Type.typ           (* deserialize value of stable type from stable memory *)
  | ICStableSize of Type.typ

(* Declarations *)

and dec = dec' Source.phrase
and dec' =
  | LetD of pat * exp                          (* immutable *)
  | VarD of id * Type.typ * exp                (* mutable *)
  | RefD of id * Type.typ * lexp               (* reference - only required for flag --experimental_field_aliasing *)

(* Literals *)

(* NB: This function is currently unused *)
let string_of_lit = function
  | BoolLit false -> "false"
  | BoolLit true  ->  "true"
  | IntLit n
  | NatLit n      -> Numerics.Int.to_pretty_string n
  | Int8Lit n     -> Numerics.Int_8.to_pretty_string n
  | Int16Lit n    -> Numerics.Int_16.to_pretty_string n
  | Int32Lit n    -> Numerics.Int_32.to_pretty_string n
  | Int64Lit n    -> Numerics.Int_64.to_pretty_string n
  | Nat8Lit n     -> Numerics.Nat8.to_pretty_string n
  | Nat16Lit n    -> Numerics.Nat16.to_pretty_string n
  | Nat32Lit n    -> Numerics.Nat32.to_pretty_string n
  | Nat64Lit n    -> Numerics.Nat64.to_pretty_string n
  | CharLit c     -> string_of_int c
  | NullLit       -> "null"
  | TextLit t     -> t
  | BlobLit b     -> Printf.sprintf "%s" b
  | FloatLit f    -> Numerics.Float.to_pretty_string f

(* Flavor *)

(*
We have a bunch of flavors of the IR, where some constructors are not
allowed in some flavors. In an ideal world, we would have different IRs for
that (or maybe GADTs). But for now we simply track that on the value level. The
main purpose of tracking that is to inform `Check_ir` about the invariants that
should hold.
*)

type flavor = {
  has_typ_field : bool; (* Typ(e) fields *)
  has_async_typ : bool; (* AsyncT *)
  has_await : bool; (* AwaitE and AsyncE *)
  has_show : bool; (* ShowE *)
  has_poly_eq : bool; (* Polymorphic equality *)
}

let full_flavor () : flavor = {
  has_typ_field = true;
  has_await = true;
  has_async_typ = true;
  has_show = true;
  has_poly_eq = true;
}

type actor_type = {
  (* original actor type, including all actor fields *)
  transient_actor_type: Type.typ;
  (* record of stable actor fields used for persistence,
     the fields are without mutability distinctions *)
  stable_actor_type: Type.typ
}

(* Program *)

type comp_unit =
  | LibU of dec list * exp
  | ProgU of dec list
  | ActorU of arg list option * dec list * field list * system * Type.typ (* actor (class) *)
     

type prog = comp_unit * flavor


(* object pattern helpers *)

let pats_of_obj_pat pfs = List.map (fun {Source.it={name; pat}; _} -> pat) pfs

let map_obj_pat f pfs =
  List.map (fun ({Source.it={name; pat}; _} as pf) -> {pf with Source.it={name; pat=f pat}}) pfs

let replace_obj_pat pfs pats =
  List.map2 (fun ({Source.it={name; pat=_}; _} as pf) pat -> {pf with Source.it={name; pat}}) pfs pats

(* Helper for transforming prims, without missing embedded typs and ids *)

let map_prim t_typ t_id p =
  match p with
  | CallPrim ts -> CallPrim (List.map t_typ ts)
  | UnPrim (ot, op) -> UnPrim (t_typ ot, op)
  | BinPrim (ot, op) -> BinPrim (t_typ ot, op)
  | RelPrim (ot, op) -> RelPrim (t_typ ot, op)
  | TupPrim
  | ProjPrim _
  | OptPrim
  | TagPrim _
  | DotPrim _
  | ActorDotPrim _ -> p
  | ArrayPrim (m, t) -> ArrayPrim (m, t_typ t)
  | IdxPrim
  | NextArrayOffset
  | EqArrayOffset
  | DerefArrayOffset
  | GetLastArrayOffset -> p
  | BreakPrim id -> BreakPrim (t_id id)
  | RetPrim
  | AwaitPrim _
  | AssertPrim
  | ThrowPrim -> p
  | ShowPrim t -> ShowPrim (t_typ t)
  | SerializePrim ts -> SerializePrim (List.map t_typ ts)
  | DeserializePrim ts -> DeserializePrim (List.map t_typ ts)
  | DeserializeOptPrim ts -> DeserializeOptPrim (List.map t_typ ts)
  | NumConvTrapPrim _
  | NumConvWrapPrim _
  | DecodeUtf8
  | EncodeUtf8 -> p
  | CastPrim (t1, t2) -> CastPrim (t_typ t1, t_typ t2)
  | ActorOfIdBlob t -> ActorOfIdBlob (t_typ t)
  | BlobOfIcUrl
  | IcUrlOfBlob -> p
  | SelfRef t -> SelfRef (t_typ t)
  | SystemTimePrim
  | SystemCyclesAddPrim
  | SystemCyclesAcceptPrim
  | SystemCyclesAvailablePrim
  | SystemCyclesBalancePrim
  | SystemCyclesRefundedPrim
  | SystemCyclesBurnPrim
  | SetCertifiedData
  | GetCertificate
  | OtherPrim _ -> p
  | CPSAwait (s, t) -> CPSAwait (s, t_typ t)
  | CPSAsync (s, t) -> CPSAsync (s, t_typ t)
  | ICReplyPrim ts -> ICReplyPrim (List.map t_typ ts)
  | ICArgDataPrim
  | ICPerformGC
  | ICRejectPrim
  | ICCallerPrim
  | ICCallPrim
  | ICCallRawPrim
  | ICMethodNamePrim -> p
  | ICStableWrite t -> ICStableWrite (t_typ t)
  | ICStableRead t -> ICStableRead (t_typ t)
  | ICStableSize t -> ICStableSize (t_typ t)
