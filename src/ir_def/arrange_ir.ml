open Mo_types
open Mo_values
open Source
open Ir
open Wasm.Sexpr

let ($$) head inner = Node (head, inner)

let id i = Atom i

let typ t = Atom (Type.string_of_typ t)
let prim_ty p = typ (Type.Prim p)
let kind k = Atom (Type.string_of_kind k)

let rec exp e = match e.it with
  | VarE (m, i)         -> (if m = Var then "VarE!" else "VarE") $$ [id i]
  | LitE l              -> "LitE"    $$ [lit l]
  | PrimE (p, es)       -> "PrimE"   $$ [prim p] @ List.map exp es
  | AssignE (le1, e2)   -> "AssignE" $$ [lexp le1; exp e2]
  | BlockE (ds, e1)     -> "BlockE"  $$ List.map dec ds @ [exp e1]
  | IfE (e1, e2, e3)    -> "IfE"     $$ [exp e1; exp e2; exp e3]
  | SwitchE (e, cs)     -> "SwitchE" $$ [exp e] @ List.map case cs
  | LoopE e1            -> "LoopE"   $$ [exp e1]
  | LabelE (i, t, e)    -> "LabelE"  $$ [id i; typ t; exp e]
  | AsyncE (Type.Fut, tb, e, t) -> "AsyncE"  $$ [typ_bind tb; exp e; typ t]
  | AsyncE (Type.Cmp, tb, e, t) -> "AsyncE*"  $$ [typ_bind tb; exp e; typ t]
  | DeclareE (i, t, e1) -> "DeclareE" $$ [id i; exp e1]
  | DefineE (i, m, e1)  -> "DefineE" $$ [id i; mut m; exp e1]
  | FuncE (x, s, c, tp, as_, ts, e) ->
    "FuncE" $$ [Atom x; func_sort s; control c] @ List.map typ_bind tp @ args as_ @ [ typ (Type.seq ts); exp e]
  | SelfCallE (ts, exp_f, exp_k, exp_r, exp_c) ->
    "SelfCallE" $$ [typ (Type.seq ts); exp exp_f; exp exp_k; exp exp_r; exp exp_c]
  | ActorE (ds, fs, u, t) -> "ActorE"  $$ List.map dec ds @ fields fs @ [system u; typ t]
  | NewObjE (s, fs, t)  -> "NewObjE" $$ (Arrange_type.obj_sort s :: fields fs @ [typ t])
  | TryE (e, cs, None) -> "TryE" $$ [exp e] @ List.map case cs
  | TryE (e, cs, Some (i, _)) -> "TryE" $$ [exp e] @ List.map case cs @ Atom ";" :: [id i]

and system { meta; preupgrade; postupgrade; heartbeat; timer; inspect; stable_record; stable_type} = (* TODO: show meta? *)
  "System" $$ [
      "Pre" $$ [exp preupgrade];
      "Post" $$ [exp postupgrade];
      "Heartbeat" $$ [exp heartbeat];
      "Timer" $$ [exp timer];
      "Inspect" $$ [exp inspect];
      "StableRecord" $$ [exp stable_record];
      "StableType" $$ [typ stable_type]
    ]

and lexp le = match le.it with
  | VarLE i             -> "VarLE" $$ [id i]
  | IdxLE (e1, e2)      -> "IdxLE" $$ [exp e1; exp e2]
  | DotLE (e1, n)       -> "DotLE" $$ [exp e1; Atom n]

and fields fs = List.fold_left (fun flds (f : field) -> (f.it.name $$ [ id f.it.var ]):: flds) [] fs

and args = function
 | [] -> []
 | as_ -> ["params" $$ List.map arg as_]

and arg a = Atom a.it

and prim = function
  | CallPrim ts       -> "CallPrim" $$ List.map typ ts
  | UnPrim (t, uo)    -> "UnPrim"     $$ [typ t; Arrange_ops.unop uo]
  | BinPrim (t, bo)   -> "BinPrim"    $$ [typ t; Arrange_ops.binop bo]
  | RelPrim (t, ro)   -> "RelPrim"    $$ [typ t; Arrange_ops.relop ro]
  | TupPrim           -> Atom "TupPrim"
  | ProjPrim i        -> "ProjPrim"   $$ [Atom (string_of_int i)]
  | OptPrim           -> Atom "OptPrim"
  | TagPrim i         -> "TagE" $$ [id i]
  | DotPrim n         -> "DotPrim" $$ [Atom n]
  | ActorDotPrim n    -> "ActorDotPrim" $$ [Atom n]
  | ArrayPrim (m, t)  -> "ArrayPrim"  $$ [mut m; typ t]
  | IdxPrim           -> Atom "IdxPrim"
  | NextArrayOffset   -> Atom "NextArrayOffset"
  | EqArrayOffset     -> Atom "EqArrayOffset"
  | DerefArrayOffset  -> Atom "DerefArrayOffset"
  | GetLastArrayOffset -> Atom "GetLastArrayOffset"
  | BreakPrim i       -> "BreakPrim"  $$ [id i]
  | RetPrim           -> Atom "RetPrim"
  | AwaitPrim Type.Fut -> Atom "AwaitPrim"
  | AwaitPrim Type.Cmp -> Atom "AwaitPrim*"
  | AssertPrim        -> Atom "AssertPrim"
  | ThrowPrim         -> Atom "ThrowPrim"
  | ShowPrim t        -> "ShowPrim" $$ [typ t]
  | SerializePrim t   -> "SerializePrim" $$ List.map typ t
  | DeserializePrim t -> "DeserializePrim" $$ List.map typ t
  | DeserializeOptPrim t -> "DeserializeOptPrim" $$ List.map typ t
  | NumConvWrapPrim (t1, t2) -> "NumConvWrapPrim" $$ [prim_ty t1; prim_ty t2]
  | NumConvTrapPrim (t1, t2) -> "NumConvTrapPrim" $$ [prim_ty t1; prim_ty t2]
  | CastPrim (t1, t2) -> "CastPrim" $$ [typ t1; typ t2]
  | DecodeUtf8        -> Atom "DecodeUtf8"
  | EncodeUtf8        -> Atom "EncodeUtf8"
  | ActorOfIdBlob t   -> "ActorOfIdBlob" $$ [typ t]
  | BlobOfIcUrl       -> Atom "BlobOfIcUrl"
  | IcUrlOfBlob       -> Atom "IcUrlOfBlob"
  | SelfRef t         -> "SelfRef"    $$ [typ t]
  | SystemTimePrim    -> Atom "SystemTimePrim"
  | SystemCyclesAddPrim -> Atom "SystemCyclesAddPrim"
  | SystemCyclesAcceptPrim -> Atom "SystemCyclesAcceptPrim"
  | SystemCyclesAvailablePrim -> Atom "SystemCyclesAvailablePrim"
  | SystemCyclesBalancePrim -> Atom "SystemCyclesBalancePrim"
  | SystemCyclesRefundedPrim -> Atom "SystemCyclesRefundedPrim"
  | SystemCyclesBurnPrim -> Atom "SystemCyclesBurnPrim"
  | SetCertifiedData  -> Atom "SetCertifiedData"
  | GetCertificate    -> Atom "GetCertificate"
  | OtherPrim s       -> Atom s
  | CPSAwait (Type.Fut, t) -> "CPSAwait" $$ [typ t]
  | CPSAwait (Type.Cmp, t) -> "CPSAwait*" $$ [typ t]
  | CPSAsync (Type.Fut, t) -> "CPSAsync" $$ [typ t]
  | CPSAsync (Type.Cmp, t) -> "CPSAsync*" $$ [typ t]
  | ICArgDataPrim     -> Atom "ICArgDataPrim"
  | ICStableSize t    -> "ICStableSize" $$ [typ t]
  | ICPerformGC       -> Atom "ICPerformGC"
  | ICReplyPrim ts    -> "ICReplyPrim" $$ List.map typ ts
  | ICRejectPrim      -> Atom "ICRejectPrim"
  | ICCallerPrim      -> Atom "ICCallerPrim"
  | ICCallPrim        -> Atom "ICCallPrim"
  | ICCallRawPrim     -> Atom "ICCallRawPrim"
  | ICMethodNamePrim  -> Atom "ICMethodNamePrim"
  | ICStableWrite t   -> "ICStableWrite" $$ [typ t]
  | ICStableRead t    -> "ICStableRead" $$ [typ t]

and mut = function
  | Const -> Atom "Const"
  | Var   -> Atom "Var"

and pat p = match p.it with
  | WildP           -> Atom "WildP"
  | VarP i          -> "VarP"       $$ [ id i ]
  | TupP ps         -> "TupP"       $$ List.map pat ps
  | ObjP pfs        -> "ObjP"       $$ List.map pat_field pfs
  | LitP l          -> "LitP"       $$ [ lit l ]
  | OptP p          -> "OptP"       $$ [ pat p ]
  | TagP (i, p)     -> "TagP"       $$ [ id i; pat p ]
  | AltP (p1,p2)    -> "AltP"       $$ [ pat p1; pat p2 ]

and lit = function
  | NullLit       -> Atom "NullLit"
  | BoolLit b     -> "BoolLit"   $$ [ Atom (if b then "true" else "false") ]
  | NatLit n      -> "NatLit"    $$ [ Atom (Numerics.Nat.to_pretty_string n) ]
  | Nat8Lit w     -> "Nat8Lit"   $$ [ Atom (Numerics.Nat8.to_pretty_string w) ]
  | Nat16Lit w    -> "Nat16Lit"  $$ [ Atom (Numerics.Nat16.to_pretty_string w) ]
  | Nat32Lit w    -> "Nat32Lit"  $$ [ Atom (Numerics.Nat32.to_pretty_string w) ]
  | Nat64Lit w    -> "Nat64Lit"  $$ [ Atom (Numerics.Nat64.to_pretty_string w) ]
  | IntLit i      -> "IntLit"    $$ [ Atom (Numerics.Int.to_pretty_string i) ]
  | Int8Lit w     -> "Int8Lit"   $$ [ Atom (Numerics.Int_8.to_pretty_string w) ]
  | Int16Lit w    -> "Int16Lit"  $$ [ Atom (Numerics.Int_16.to_pretty_string w) ]
  | Int32Lit w    -> "Int32Lit"  $$ [ Atom (Numerics.Int_32.to_pretty_string w) ]
  | Int64Lit w    -> "Int64Lit"  $$ [ Atom (Numerics.Int_64.to_pretty_string w) ]
  | FloatLit f    -> "FloatLit"  $$ [ Atom (Numerics.Float.to_pretty_string f) ]
  | CharLit c     -> "CharLit"   $$ [ Atom (string_of_int c) ]
  | TextLit t     -> "TextLit"   $$ [ Atom t ]
  | BlobLit b     -> "BlobLit"   $$ [ Atom (Printf.sprintf "%S" b) ] (* hex might be nicer *)

and pat_field pf = pf.it.name $$ [pat pf.it.pat]

and case c = "case" $$ [pat c.it.pat; exp c.it.exp]

and func_sort s = Atom (Arrange_type.func_sort s)

and control s = Atom (Arrange_type.control s)

and dec d = match d.it with
  | LetD (p, e) -> "LetD" $$ [pat p; exp e]
  | VarD (i, t, e) -> "VarD" $$ [id i; typ t; exp e]
  | RefD (i, t, e) -> "RefD" $$ [id i; typ t; lexp e]

and typ_bind (tb : typ_bind) =
  Type.string_of_con tb.it.con $$ [typ tb.it.bound]

and comp_unit = function
  | LibU (ds, e) -> "LibU" $$ List.map dec ds @ [ exp e ]
  | ProgU ds -> "ProgU" $$ List.map dec ds
  | ActorU (None, ds, fs, u, t) -> "ActorU"  $$ List.map dec ds @ fields fs @ [system u; typ t]
  | ActorU (Some as_, ds, fs, u, t) -> "ActorU"  $$ List.map arg as_ @ List.map dec ds @ fields fs @ [system u; typ t]

and prog (cu, _flavor) = comp_unit cu
