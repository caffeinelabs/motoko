// The right-hand side of `??` is expression position: a block needs `do { ... }` (M0273).
let o : ?Nat = null;
let z = o ?? { let d = 1; d };
