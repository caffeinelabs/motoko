// Exercise incompatible_funcs: type parameter bound mismatch in generic functions.
// This triggers when rel_binds fails (None) in the Func case of rel_typ.
// Also exercises incompatible_async_sorts (Fut vs Cmp), incompatible_cons,
// and the general incompatible_types path.

// Type parameter bounds mismatch: <T <: Nat> vs <T <: Text>
// These two function types cannot be unified
type F1 = <T <: Nat>(T) -> T;
type F2 = <T <: Text>(T) -> T;
let _x : F1 = (func<T <: Text>(x : T) : T = x : F2);
