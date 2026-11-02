// Regression test: a shared function stored in a mutable (aliased) position is
// checked for memory compatibility under type invariance. The memoization cache
// used to be indexed with the old/new type tables transposed for function
// parameters under invariance, which could mark an unrelated pair visited or,
// as here, index the cache out of bounds once the two versions have different
// type-table sizes. A debug RTS build trips the `i < size1` assertion in
// `bitrel.rs`; see `rts/motoko-rts/src/idl.rs`.
//MOC-FLAG --sanity-checks
actor {
  // `[var ...]` makes the element function type sit under an alias, so its
  // parameter record `{ x : Nat }` is compared under invariance.
  stable var callbacks : [var (shared { x : Nat } -> ())] = [var];
};
