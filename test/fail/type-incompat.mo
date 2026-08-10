// incompatible_* diagnostics (M0096) across rel_typ: prim, object sort, function
// sort, function control, generic-function binds, async sort/scope. do-isolated.
do { let _ : Text = (42 : Nat) };
do { module M { public let x : Nat = 42 }; let _ : {x : Nat} = M };
do { let f : shared Nat -> async Nat = func(n : Nat) : async Nat = async n; let g : Nat -> async Nat = f };
do { type F1 = <T <: Nat>(T) -> T; type F2 = <T <: Text>(T) -> T; let _x : F1 = (func<T <: Text>(x : T) : T = x : F2) };
do { let f : shared () -> async Nat = func() : async Nat = async 0; let _ : shared () -> async Text = f };
do { let f : shared () -> () = func() {}; let g : shared Nat -> async Nat = func(n : Nat) : async Nat = async n; let _ : shared () -> async () = func() {} };
