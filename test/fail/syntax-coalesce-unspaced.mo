// `??` without trailing whitespace means two option introductions,
// so it cannot be used as the binary null-coalescing operator.
let o : ?Nat = null;
let x = o ??0;
