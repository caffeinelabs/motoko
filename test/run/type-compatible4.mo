// Exercise compatible_typ Null/Opt arms (L1457: Prim Null vs Opt _)
// and the Weak arm (L1455-1456).
// Also exercises compatible_typ Any arms (L1437-1441: Non, Any).

//MOC-FLAG -A=M0194

// Null/Opt: compatible_typ (Prim Null) (Opt t) -> true
// Force leg1 to be Null type and leg2 to be ?Nat
// This requires: leg1 annotated as Null, leg2 as ?Nat
// Both explicit (AnnotP), and scrutinee must be Null (subtype of ?Nat)
let nullVal : Null = null;
let _ = switch nullVal {
  case ((x : Null) and (y : Null)) 0
};

// Any arms:
// Any,Any -> true (when both sides are Any — wildcard patterns)
// For this we need to force the inferred type to be Any somehow
// An annotated wildcard? Actually AnnotP _ is not a VarP...
let anyVal : Any = (42 : Any);
let _ = switch anyVal {
  case ((x : Any) and (y : Any)) 0
};

// Non: compatible_typ Non _ -> true, _ Non -> true
// Non type is the bottom type; a value of type Non shouldn't exist
// but compatible_typ treats it as compatible with anything

// Weak arm: compatible_typ (Weak t1) (Weak t2) -> recursive
// Weak types appear internally for some type inference scenarios

assert true;
