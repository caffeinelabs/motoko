// Exercise incompatible_async_sorts (Fut vs Cmp mismatch in Async type)
// and incompatible_async_scopes (different scope type variables).

// Fut async vs different scope: create a mismatch via type annotation
// Async(Fut, scope, t) vs Async(Fut, scope', t) — scope mismatch hits incompatible_async_scopes

// This forces the async scope type to mismatch
let f : shared () -> async Nat = func() : async Nat = async 0;
let _ : shared () -> async Text = f;  // Nat vs Text in async — incompatible_types in async
