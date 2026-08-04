import Prim "mo:⛔";

// Candid rule `service <actortype> <: principal`:
// service references decode at type `principal`, but not vice versa.

type S = actor { m : shared () -> async S };
type P = actor { m : shared () -> async Principal };

let a = actor "w7x7r-cok77-xa" : actor {};
let s = actor "w7x7r-cok77-xa" : S;
let sp = actor "w7x7r-cok77-xa" : P;

// a service reference value decodes at principal
do {
  let p : ?Principal = from_candid (to_candid (a));
  assert p == ?Prim.principalOfActor a;
};

// a recursive service type still decodes at principal
do {
  let p : ?Principal = from_candid (to_candid (s));
  assert p == ?Prim.principalOfActor s;
};

// a principal value does not decode at a service type
do {
  let x : ?(actor {}) = from_candid (to_candid (Prim.principalOfActor a));
  assert x == null;
};

// deferred subtype check: func () -> (service ...) decodes at func () -> (principal)
do {
  let g : ?(shared () -> async Principal) = from_candid (to_candid (s.m));
  assert g == ?(sp.m);
};

// ... but func () -> (principal) does not decode at func () -> (service ...)
do {
  let g : ?(shared () -> async S) = from_candid (to_candid (sp.m));
  assert g == null;
};

//SKIP run
//SKIP run-ir
//SKIP run-low
