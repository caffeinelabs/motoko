open Mo_def
open Mo_types
open Mo_values

open Syntax
open Source

module T = Type
module V = Value

module ValSet = Set.Make(struct type t = V.value let compare = V.compare end)
module TagSet = Set.Make(struct type t = string let compare = String.compare end)
module LabMap = Map.Make(struct type t = string let compare = String.compare end)
module AtSet = Set.Make(struct type t = Source.region let compare = compare end)

type desc =
  | Any
  | Val of V.value
  | NotVal of ValSet.t
  | Tup of desc list
  | Obj of desc LabMap.t
  | Opt of desc
  | Tag of desc * string
  | NotTag of TagSet.t

type ctxt =
  | InOpt of ctxt
  | InTag of ctxt * string
  | InTup of ctxt * desc list * desc list * pat list * T.typ list
  | InObj of ctxt * desc LabMap.t * string * pat_field list * T.field list
  | InAlt1 of ctxt * Source.region * pat * T.typ
  | InAlt2 of ctxt * Source.region
  | InCase of Source.region * case list * T.typ

type sets =
  { mutable cases : AtSet.t;
    mutable alts : AtSet.t;
    mutable reached_cases : AtSet.t;
    mutable reached_alts : AtSet.t;
    mutable missing : desc list;
  }


let make_sets () =
  { cases = AtSet.empty;
    alts = AtSet.empty;
    reached_cases = AtSet.empty;
    reached_alts = AtSet.empty;
    missing = [];
  }


(* Generating counter examples *)

let max_expand = 2

let pick_nat (type t) (module Num : Numerics.NumType with type t = t) to_val vs =
  let x = ref Num.zero in
  while ValSet.mem (to_val !x) vs do
    x := Num.(add one !x)
  done;
  Val (to_val !x)

let pick_int (type t) (module Num : Numerics.NumType with type t = t) to_val vs =
  let x = ref Num.zero in
  while ValSet.mem (to_val !x) vs do
    x := Num.neg !x;
    Num.(if ge !x zero then x := add one !x)
  done;
  Val (to_val !x)

let pick_char vs =
  let x = ref 0 in
  while ValSet.mem (V.Char !x) vs do
    x := !x + 1
  done;
  Val (V.Char !x)

let pick_val vs = function
  | T.Null -> Val V.Null
  | T.Bool -> Val (V.Bool (ValSet.mem (V.Bool false) vs))
  | T.Nat -> pick_nat (module Numerics.Nat) (fun x -> V.Int x) vs
  | T.Nat8 -> pick_nat (module Numerics.Nat8) (fun x -> V.Nat8 x) vs
  | T.Nat16 -> pick_nat (module Numerics.Nat16) (fun x -> V.Nat16 x) vs
  | T.Nat32 -> pick_nat (module Numerics.Nat32) (fun x -> V.Nat32 x) vs
  | T.Nat64 -> pick_nat (module Numerics.Nat64) (fun x -> V.Nat64 x) vs
  | T.Int -> pick_int (module Numerics.Int) (fun x -> V.Int x) vs
  | T.Int8 -> pick_int (module Numerics.Int_8) (fun x -> V.Int8 x) vs
  | T.Int16 -> pick_int (module Numerics.Int_16) (fun x -> V.Int16 x) vs
  | T.Int32 -> pick_int (module Numerics.Int_32) (fun x -> V.Int32 x) vs
  | T.Int64 -> pick_int (module Numerics.Int_64) (fun x -> V.Int64 x) vs
  | T.Char -> pick_char vs
  | T.Text
  | T.Blob
  | T.Error
  | T.Principal
  | T.Region
  | T.Float
  | T.Float32 -> Any

let rec expand_notval t n vs : desc list =
  let missing = Lib.Option.get (T.span t) max_int - ValSet.cardinal vs in
  if missing = 0 then [] else
  if n = max_expand && missing > 1 then [Any] else
  match t with
  | T.Prim t' ->
    (match pick_val vs t' with
    | Val v -> Val v :: expand_notval t (n + 1) (ValSet.add v vs)
    | _ -> [Any]
    )
  | T.Opt _ -> [Opt Any]
  | _ -> [Any]


let rec pick_tag ls = function
  | [] -> assert false
  | tf::tfs when TagSet.mem tf.T.lab ls -> pick_tag ls tfs
  | tf::_ -> tf.T.lab

let rec expand_nottag tfs n ls : desc list =
  let missing = List.length tfs - TagSet.cardinal ls in
  if missing = 0 then [] else
  if n = max_expand && missing > 1 then [Any] else
  let l = pick_tag ls tfs in
  Tag (Any, l) :: expand_nottag tfs (n + 1) (TagSet.add l ls)

(* TODO: pretty print *)
let rec string_of_desc t = function
  | Any -> "_"
  | Val v -> V.string_of_val 100 t v
  | NotVal vs -> string_of_descs t (expand_notval (T.promote t) 0 vs)
  | Tup descs ->
    let ts = T.as_tup_sub (List.length descs) t in
    "(" ^ String.concat ", " (List.map2 string_of_desc ts descs) ^ ")"
  | Obj ldescs ->
    let fields = LabMap.bindings ldescs in
    (* TODO: Printing explodes, because it can't look up the type field otherwise *)
    let fields = List.filter (fun (_, desc) -> desc <> Any) fields in
    let _, tfs = T.as_obj_sub (List.map fst fields) t in
    "{" ^ String.concat "; " (List.map (string_of_ldesc tfs) fields) ^ "}"
  | Opt desc ->
    let t' = T.as_opt_sub t in
    "?(" ^ string_of_desc t' desc ^ ")"
  | Tag (desc, l) ->
    let t' = T.lookup_val_field l (T.as_variant_sub l t) in
    if T.sub t' T.unit then "#" ^ l
    else if T.is_tup t' then "#" ^ l ^ " " ^ string_of_desc t' desc
    else "#" ^ l ^ "(" ^ string_of_desc t' desc ^ ")"
  | NotTag ls ->
    let tfs = T.as_variant (T.promote t) in
    string_of_descs t (expand_nottag tfs 0 ls)

and string_of_ldesc tfs (l, desc) =
  l ^ " = " ^ string_of_desc (T.lookup_val_field l tfs) desc

and string_of_descs t descs =
  assert (descs <> []);
  String.concat " or " (List.map (string_of_desc t) descs)


(* Abstract interpretation *)

let is_neg_int = function
  | V.Int i -> Numerics.Int.(lt i zero)
  | _ -> false

let value_of_lit = function
  | NullLit -> V.Null
  | BoolLit b -> V.Bool b
  | NatLit n -> V.Int n
  | Nat8Lit w -> V.Nat8 w
  | Nat16Lit w -> V.Nat16 w
  | Nat32Lit w -> V.Nat32 w
  | Nat64Lit w -> V.Nat64 w
  | IntLit i -> V.Int i
  | Int8Lit w -> V.Int8 w
  | Int16Lit w -> V.Int16 w
  | Int32Lit w -> V.Int32 w
  | Int64Lit w -> V.Int64 w
  | FloatLit z -> V.Float z
  | Float32Lit z -> V.Float32 z
  | CharLit c -> V.Char c
  | TextLit t -> V.Text t
  | BlobLit b -> V.Blob b
  | PreLit _ -> assert false


let (&&&) = (&&) (* No short-cutting *)

let skip_pat at sets =
  sets.alts <- AtSet.add at sets.alts;
  true

let rec match_pat ctxt desc pat t sets =
  T.span t = Some 0 && skip_pat pat.at sets ||
  match pat.it with
  | WildP | VarP _ ->
    if T.inhabited t then
      succeed ctxt desc sets
    else
      skip_pat pat.at sets
  | LitP lit ->
    match_lit ctxt desc pat.at (value_of_lit !lit) t sets
  | SignP (op, lit) ->
    let f = Operator.unop op (Operator.type_unop op pat.note) in
    match_lit ctxt desc pat.at (f (value_of_lit !lit)) t sets
  | TupP pats ->
    let ts = T.as_tup (T.promote t) in
    let descs =
      match desc with
      | Tup descs -> descs
      | Any -> List.map (fun _ -> Any) pats
      | _ -> assert false
    in match_tup ctxt [] descs pats ts sets
  | ObjP pat_fields ->
    let _, tfs = T.as_obj (T.promote t) in
    let ldescs =
      match desc with
      | Obj ldescs -> ldescs
      | Any ->
        LabMap.(List.fold_left
          (fun m (tf : T.field) -> add tf.T.lab Any m) empty tfs)
      | _ -> assert false
    in match_obj ctxt ldescs pat_fields tfs sets
  | OptP pat1 ->
    if T.is_prim T.Null (T.promote t) then  (* may occur through subtyping *)
      skip_pat pat.at sets && fail ctxt (Val V.Null) sets
    else
      let t' = T.as_opt (T.promote t) in
      (match desc with
      | Opt desc' ->
        match_pat (InOpt ctxt) desc' pat1 t' sets
      | Val V.Null ->
        fail ctxt desc sets
      | NotVal vs when ValSet.mem V.Null vs ->
        match_pat (InOpt ctxt) Any pat1 t' sets
      | Any ->
        fail ctxt (Val V.Null) sets &&&
        match_pat (InOpt ctxt) Any pat1 t' sets
      | _ -> assert false
      )
  | TagP (id, pat1) ->
    let t', found =
      match T.lookup_val_field_opt id.it (T.as_variant (T.promote t)) with
      | None -> T.Non, false  (* may occur through subtyping *)
      | Some t' -> t', true
    in
    (match desc with
    | Tag (desc', l) ->
      if id.it = l then
        match_pat (InTag (ctxt, l)) desc' pat1 t' sets
      else
        fail ctxt desc sets
    | NotTag ls ->
      if TagSet.mem id.it ls then
        fail ctxt desc sets
      else if not found then
        skip_pat pat.at sets && fail ctxt desc sets
      else if T.span t = Some (TagSet.cardinal ls + 1) then
        match_pat (InTag (ctxt, id.it)) Any pat1 t' sets
      else
        fail ctxt (NotTag (TagSet.add id.it ls)) sets &&&
        match_pat (InTag (ctxt, id.it)) Any pat1 t' sets
    | Any ->
      match_pat ctxt (NotTag TagSet.empty) pat t sets
    | _ -> assert false
    )
  | AltP (pat1, pat2) ->
    sets.alts <- AtSet.add pat1.at (AtSet.add pat2.at sets.alts);
    match_pat (InAlt1 (ctxt, pat1.at, pat2, t)) desc pat1 t sets
  | AnnotP (pat1, _)
  | ParP pat1 ->
    match_pat ctxt desc pat1 t sets

and match_lit ctxt desc at v t sets =
  match desc with
  | Val v' ->
    if V.equal v v' then
      succeed ctxt desc sets
    else
      fail ctxt desc sets
  | NotVal vs ->
    if ValSet.mem v vs then
      fail ctxt desc sets
    else if T.eq t T.nat && is_neg_int v then  (* may occur through subtyping *)
      skip_pat at sets && fail ctxt desc sets
    else if T.span t = Some (ValSet.cardinal vs + 1) then
      succeed ctxt (Val v) sets
    else
      fail ctxt (NotVal (ValSet.add v vs)) sets &&&
      succeed ctxt (Val v) sets
  | Opt _ ->
    fail ctxt desc sets
  | Any ->
    match_lit ctxt (NotVal ValSet.empty) at v t sets
  | _ ->
    assert false

and match_tup ctxt descs_r descs pats ts sets =
  match descs, pats, ts with
  | [], [], [] ->
    succeed ctxt (Tup (List.rev descs_r)) sets
  | desc::descs', pat::pats', t::ts' ->
    match_pat (InTup (ctxt, descs_r, descs', pats', ts')) desc pat t sets
  | _ ->
    assert false

and match_obj ctxt ldescs (pat_fields : pat_field list) tfs sets =
  match pat_fields with
  | [] -> succeed ctxt (Obj ldescs) sets
  | {it = ValPF(id, p); _}::pat_fields' ->
    let l = id.it in
    let typ = T.lookup_val_field l tfs in
    let desc = LabMap.find l ldescs in
    match_pat (InObj (ctxt, ldescs, l, pat_fields', tfs)) desc p typ sets
  | {it = TypPF(_); _}::pat_fields' ->
     match_obj ctxt ldescs pat_fields' tfs sets

and succeed ctxt desc sets : bool =
  match ctxt with
  | InOpt ctxt' ->
    succeed ctxt' (Opt desc) sets
  | InTag (ctxt', l) ->
    succeed ctxt' (Tag (desc, l)) sets
  | InTup (ctxt', descs_r, descs, pats, ts) ->
    match_tup ctxt' (desc::descs_r) descs pats ts sets
  | InObj (ctxt', ldescs, l, pfs, tfs) ->
    match_obj ctxt' (LabMap.add l desc ldescs) pfs tfs sets
  | InAlt1 (ctxt', at1, _pat2, _t) ->
    sets.reached_alts <- AtSet.add at1 sets.reached_alts;
    succeed ctxt' desc sets
  | InAlt2 (ctxt', at2) ->
    sets.reached_alts <- AtSet.add at2 sets.reached_alts;
    succeed ctxt' desc sets
  | InCase (at, cases, _t) ->
    sets.reached_cases <- AtSet.add at sets.reached_cases;
    skip cases sets

and skip cases sets : bool =
  match cases with
  | [] ->
    true
  | case::cases' ->
    sets.cases <- AtSet.add case.it.pat.at sets.cases;
    skip cases' sets

and fail ctxt desc sets : bool =
  match ctxt with
  | InOpt ctxt' ->
    fail ctxt' (Opt desc) sets
  | InTag (ctxt', l) ->
    fail ctxt' (Tag (desc, l)) sets
  | InTup (ctxt', descs', descs, pats, _ts) ->
    fail ctxt' (Tup (List.rev descs' @ [desc] @ descs)) sets
  | InObj (ctxt', ldescs, l, pats, _tfs) ->
    fail ctxt' (Obj (LabMap.add l desc ldescs)) sets
  | InAlt1 (ctxt', at1, pat2, t) ->
    match_pat (InAlt2 (ctxt', pat2.at)) desc pat2 t sets
  | InAlt2 (ctxt', at2) ->
    fail ctxt' desc sets
  | InCase (at, [], t) ->
    T.span t = Some 0 || not (T.inhabited t) ||
    (sets.missing <- desc::sets.missing; false)
  | InCase (at, case::cases, t) ->
    T.span t = Some 0 && skip (case::cases) sets ||
    match_pat (InCase (case.it.pat.at, cases, t)) desc case.it.pat t sets


type uncovered = string
type unreached = Source.region

let check_cases cases t =
  let sets = make_sets () in
  let _exhaustive = fail (InCase (Source.no_region, cases, t)) Any sets in
  let uncovered = List.map (string_of_desc t) (List.rev sets.missing) in
  let unreached_cases = AtSet.diff sets.cases sets.reached_cases in
  let unreached_alts = AtSet.diff sets.alts sets.reached_alts in
  uncovered, AtSet.elements (AtSet.union unreached_cases unreached_alts)

let (@?) it at = {it; at; note = empty_typ_note}

let check_pat pat t =
  let uncovered, unreached =
    check_cases [{pat; exp = TupE [] @? Source.no_region} @@ Source.no_region] t
  in uncovered, List.filter ((<>) pat.at) unreached
