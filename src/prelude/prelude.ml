let prelude = [%blob "prelude/prelude.mo"]
let internals = [%blob "prelude/internals.mo"]
let timers_api = [%blob "prelude/timers-api.mo"]
let prim_module' = [%blob "prelude/prim.mo"]
(* The actor* address primitives, added to Prim only under --experimental-local-actors, so a
   program without the flag sees exactly the usual Prim. `(actor* {}) -> ...` is
   parenthesised because the grammar does not take an object-sort type as a bare function
   domain (`actor {} -> Nat` is M0001 too). *)
let local_actors_api =
  "func localActorAddress(value : actor* {}) : {container : Principal; slot : Nat64; generation : Nat64} = (prim \"local_actor_address\" : (actor* {}) -> {container : Principal; slot : Nat64; generation : Nat64})(value);\n" ^
  "func localActorFromAddress<A <: actor* {}>(address : {container : Principal; slot : Nat64; generation : Nat64}) : A = (prim \"local_actor_from_address\" : {container : Principal; slot : Nat64; generation : Nat64} -> A)(address);\n"
let prim_module ?(local_actors = false) ~timers:required () =
  let m = if required then prim_module' ^ timers_api else prim_module' in
  if local_actors then m ^ local_actors_api else m
