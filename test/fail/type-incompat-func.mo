// Exercise incompatible_func_sorts and incompatible_func_controls in rel_typ.
// incompatible_func_sorts: Local vs Shared function sort mismatch
// Triggers when a local function is passed where a shared one is expected.

// Function sort mismatch: local vs shared query
let f : shared Nat -> async Nat = func(n : Nat) : async Nat = async n;
let g : Nat -> async Nat = f;  // shared <: local? No — should work only locally
