ignore ({ foo = 5 * 5 } : { foo : Nat64 }); // accept issue #2240

ignore ({ a = 0; var v = 0 } : { a : Int; var v : Int }); // accept

let r : { a : Int; var v : Int } = { a = 0; var v = 0 }; // accept

ignore ({ v = 0 } : { var v : Nat }); // reject mutability mismatch

ignore ({ var a = 0 } : { a : Nat }); // reject mutability mismatch

ignore ({ a = 0; var v = 0 } : { var a : Int; v : Int }); // reject, mutability mismatches

ignore ({} : { a : Nat }); // reject

ignore ({ a = 0; b = 1 } : { a : Nat }); // accept (but warn in the future?)

do {
  let a = 1;
  let b = 2;
  { a = a; b = a; c = 1; d = c }; // reject (c not bound)
};

ignore ({ {} with a = 1 } : { b : Nat }); // reject
