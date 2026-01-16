open ExpGraph
open Type

(*
This module exports a function

  typ_hash : typ -> string

with these properties
 - Its domain are closed types
 - It does not support abstract types (for now)
 - It does not support type parameters (for now)
 - Type.eq t1 t2 = (typ_hash t1 = typ_hash t2)
 - The output is readable yet compact


The output format does not matter hugely (equality is what counts), but here is
a little reading advice:

 * Nullary types are described a single letter, or few letters:

     Nat: N
     Int8: i8
     (): u

 * Unary type constructors just prefix their type argument

     ?Nat: ?N
     [Blob]: vB
     [Mut Text]: Vt

 * Tuples put parentheses around the component types, without separator:

     (Int, ?Bool, Text) = (I?bt)

 * Objects and variants have comma-separated lists of label-type-pairs:

     {foo : Int; var bar : Text} = r(foo:I,bar!:t)
     {#tag : Int} = v(tag:I)

   The various object forms are indicated after the r:

     module {foo : Int; var bar : Text} = rm(foo:I,bar!:t)

 * Functions:

     shared (Int, Bool) -> Text = Fs(Ib)(t)

 * Types that occur more than once are prefixed with a number when seen first,
   and later referenced by that number:

     ([mut Int]) -> ([mut Int]) = F(1=VI)(!1)

     type List = ?(Nat,List)
     (List, ?List) = (1=?(N!1)?!1)

     type List<A> = ?(Nat,List<A>)
     List<N> = 1=?(N!1)

For this encoding to be injective, it must be prefix free, i.e.

  ∀ t1 t2 s. typ_hash t1 = (typ_hash t2 ^ s) ⟹ eq t1 t2 = true

*)

(* A hint about how to print the node *)
type arity = Nullary | Unary | Nary | Labeled of string list | TwoSeq of int

(*
Type construtors are (mostly) described by a single letter.
NB: This code better be prefix-free. Hence capital N, otherwise n8=u
is a bit ambigous
*)

let prim = function
  | Null -> "z"
  | Bool -> "b"
  | Nat -> "N"
  | Nat8 -> "n8"
  | Nat16 -> "n16"
  | Nat32 -> "n32"
  | Nat64 -> "n64"
  | Int -> "I"
  | Int8 -> "i8"
  | Int16 -> "i16"
  | Int32 -> "i32"
  | Int64 -> "i64"
  | Float -> "f"
  | Char -> "c"
  | Text -> "t"
  | Blob -> "B"
  | Error -> "E"
  | Principal -> "P"
  | Region -> "R"

let rec go = function
  | Prim p -> ((Nullary, prim p), [])
  | Any -> ((Nullary, "a"), [])
  | Non -> ((Nullary, "e"), []) (* e for empty *)
  | Opt t -> ((Unary, "?"), [t])
  | Tup [] -> ((Nullary, "u"), [])
  | Tup ts -> ((Nary, ""), ts)
  | Array (Mut t) -> ((Unary, "V"), [t])
  | Array t -> ((Unary, "v"), [t])
  | Weak t -> ((Unary, "w"), [t])

  (* Here we pretend we support first-class mutable values;
     this is useful for stable serialization *)
  | Mut t -> ((Unary, "M"), [t])

  | Obj (s, fs, tfs) ->
    ( ( Labeled (List.map (fun f -> f.lab ^ if is_mut f.typ then "!" else "") fs @ List.map (fun f -> f.lab) tfs),
        (match s with Object -> "r" | Module -> "rm" | Memory -> "rs" | Actor -> "ra" | Mixin -> "rx")
      )
    , List.map (fun f -> as_immut f.typ) fs
    )
  | Variant fs ->
    ( ( Labeled (List.map (fun f -> f.lab) fs),
        "v"
      )
    , List.map (fun f -> f.typ) fs
    )

  | Func (s, c, tbs, ts1, ts2) ->
    List.iter (fun bind -> assert (bind.sort = Scope)) tbs;
    ( ( TwoSeq (List.length ts1),
        "F" ^
        (match s with Local -> "" | Shared Query -> "q" | Shared Write -> "s" | Shared Composite -> "C") ^
        (match c with Returns -> "" | Promises -> "p" | Replies -> "r")
      )
    , ts1 @ ts2
    )
  | Async _ -> raise (Invalid_argument "typ_hash: Only supports serializable data")
  | Con _ as t -> go (normalize t)
  | Named _ as t -> go (normalize t)
  | Pre -> assert false
  | Var _ -> assert false

let paren xs = "(" ^ String.concat "" xs ^ ")"

let of_con (a, k) args =
  match a, args with
  | Nullary, _ ->
    assert (args = []);
    k
  | Unary, [a] ->
    k ^ a
  | Unary, _ ->
    assert false
  | Nary, _ ->
    k ^ paren args
  | Labeled ls, _ ->
    k ^ "(" ^ String.concat "," (List.map2 (fun l a -> l ^ ":" ^ a) ls args) ^ ")"
  | TwoSeq n, _->
    let (a1, a2) = Lib.List.split_at n args in
    k ^ paren a1 ^ paren a2

let of_ref i x =
  Int.to_string i ^ "=" ^ x

let of_def i =
  "!" ^ Int.to_string i

let typ_hash : typ -> string =
  fun t -> t |> unfold go |> canonicalize |> fold of_con of_ref of_def

let typ_seq_hash : typ list -> string = fun ts ->
  String.concat "" (List.map typ_hash ts)


(* Some small unit tests *)

[@@@warning "-32"]
let test t expected =
  let actual = typ_hash t in
  if actual = expected then
    true
  else
    (Printf.printf "\nExpected:\n  %s\nbut got:\n  %s\n" expected actual; false)

let%test "monolist" =
  let con = Cons.fresh "List" (Abs ([], Pre))  in
  let t = Con (con, []) in
  Cons.unsafe_set_kind con (Def ([], Opt (Tup [nat; t])));
  test t "0=?(N!0)"

let%test "polylist" =
  let con = Cons.fresh "List" (Abs ([], Pre))  in
  let bind = { var = "T"; sort = Type; bound = Any } in
  let v = Var ("T", 0) in
  Cons.unsafe_set_kind con (Def ([bind], Opt (Tup [v; Con (con, [v])])));
  let t = Con (con, [nat]) in
  test t "0=?(N!0)"
