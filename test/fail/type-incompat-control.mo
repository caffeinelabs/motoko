// Exercise incompatible_func_controls: Returns vs Replies mismatch.
// A function marked as `oneway` (no return) vs a normal function.

// Control mismatch: func with Returns vs Replies (oneway)
// `system` functions use Replies/Promises; mismatch triggers incompatible_func_controls

// Direct control mismatch via annotation
let f : shared () -> () = func() {};  // Returns
let g : shared Nat -> async Nat = func(n : Nat) : async Nat = async n;

// Type annotation forces a specific control type
let _ : shared () -> async () = func() {};  // local -> async mismatch
