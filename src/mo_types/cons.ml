(*
The kind field is a reference to break the recursion in open_binds,
and to allow the multiple passes in typing. These go through Type.set_kind
with additional safeguards.

Besides these two use-cases, the kind should not be mutated, and treated like
immutable data.

This module interface guarantees that constructors with the same stamp have the
same ref.
*)

type scope = string

type 'a con = {name : string;
               stamp : int * scope option;
               hash: int; (* hash of name, stamp *)
               kind : 'a ref}

type 'a t = 'a con

module Stamps = Env.Make (struct
  type t = string * scope option

  let compare = Stdlib.compare
end)

type stamps = {stamps : int Stamps.t; scope : scope option}

let stamps : stamps ref = ref {stamps = Stamps.empty; scope = None}

let session ?scope f =
  let original = !stamps in
  stamps := {!stamps with scope};
  Fun.protect ~finally:(fun _ -> stamps := original) f

let fresh_stamp name =
  let scope = !stamps.scope in
  let n = Lib.Option.get (Stamps.find_opt (name, scope) !stamps.stamps) 0 in
  stamps := {!stamps with stamps = Stamps.add (name, scope) (n + 1) !stamps.stamps};
  n, scope

let hash name stamp = Hashtbl.hash (name, stamp)

let fresh name k =
  let stamp = fresh_stamp name in
  {name; stamp;
   hash = hash name stamp;
   kind = ref k}
let clone c k =
  let name = c.name in
  let stamp = fresh_stamp c.name in
  {name;
   stamp;
   hash = hash name stamp;
   kind = ref k}

let bump_stamps_past cs =
  List.iter (fun c ->
    let (n, scope) = c.stamp in
    let key = (c.name, scope) in
    let cur = Lib.Option.get (Stamps.find_opt key !stamps.stamps) 0 in
    if n >= cur then
      stamps := { !stamps with stamps = Stamps.add key (n + 1) !stamps.stamps }
  ) cs

let kind c = !(c.kind)
let unsafe_set_kind c k = c.kind := k

let name c = c.name

let to_string show_stamps sep c =
  if not show_stamps || c.stamp = (0, Some "prelude")
  then c.name else Printf.sprintf "%s%s%i" c.name sep c.hash

let compare c1 c2 =
  match Int.compare c1.hash c2.hash with
  | 0 ->
    (match Int.compare (fst c1.stamp) (fst c2.stamp) with
     | 0 ->
       (match Option.compare String.compare (snd c1.stamp) (snd c2.stamp) with
        | 0 -> String.compare c1.name c2.name
        | ord -> ord)
     | ord -> ord)
  | ord -> ord

let eq c1 c2 = compare c1 c2 = 0
