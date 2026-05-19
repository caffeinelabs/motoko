// M12 negative case: wire data encoded as one GADT instantiation,
// decoded as a different one. The pruned wire variants don't share
// the offending arm, so the Candid decoder's variant-tag lookup
// fails, and `from_candid : ?T` returns null.

type Expr<A> = {
  #int  : type A = Nat in Nat;
  #bool : type A = Bool in A;
};

// Sender's wire format for `Expr<Nat>` is pruned to `{#int : Nat}` —
// the `#bool` arm is unreachable for A=Nat and dropped.
let e_nat : Expr<Nat> = #int 42;
let blob = to_candid (e_nat);

// Receiver expects `Expr<Bool>` whose pruned wire form is
// `{#bool : Bool}` — `#int` is gone. The decoded tag-hash for `#int`
// has no match in the receiver's pruned variant, so the Candid
// deserialiser produces a coercion error → `null` in opt mode.
let back : ?Expr<Bool> = from_candid blob;
switch back {
  case null { /* expected: wire's tag is not in receiver's pruned variant */ };
  case (?_) assert false;
};

//SKIP run
//SKIP run-ir
//SKIP run-low
