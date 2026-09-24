//MOC-FLAG --all-libs
//MOC-FLAG --package core ../core-stub/src

// The import hint for a candidate found in a nested module must point at the
// importable lib (mo:core/Nested), not at the nested path inside it.

func apply(x : Nat, nestedDouble : (implicit : Nat -> Nat)) : Nat {
  nestedDouble(x);
};

ignore apply(3);
