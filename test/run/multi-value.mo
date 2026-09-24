func returns_tuple() : (Nat, Nat) = (1,2);

assert ((if true { returns_tuple() } else { returns_tuple() }) == (1,2));

func pair(n : Nat64) : (Nat64, Nat64) = (n, n + 1);
let (a, b) = pair(42);
assert (a == 42 and b == 43);

// Multi-value codegen (now the default): a tuple-returning helper emits a
// multi-result signature instead of stashing values through `multi_val_*`
// globals.
//CHECK: func $pair{{.*}}(result i{{32|64}} i{{32|64}})
//CHECK-NOT: multi_val_
