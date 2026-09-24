//MOC-FLAG --package core ../core-stub/src
//MOC-FLAG --implicit-package core

// Nested candidates from implicit-package libs must resolve without an
// explicit import: core/Nested is never imported here, and the candidate
// lives in its nested module Inner.

func apply(x : Nat, nestedDouble : (implicit : Nat -> Nat)) : Nat {
  nestedDouble(x);
};

assert apply(3) == 6;
