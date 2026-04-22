import Prim "mo:prim";

let n1 = null;
let n2 : ?Int = null;
let nat = ?5;
let nn0 = null;
let nn1 = ?null;
let nn2 : ??Nat = ?null;
let nn3 = ??1;
let nn4 : ??Nat = ??1;

// '?? T' (with whitespace) must still parse as nested option, for backward
// compatibility with the pre-PR tokenization of `??` as two `?`.
let nn5 : ?? Nat = ?? 1;
assert (nn5 == ?? 1);
switch (?? 9 : ?? Nat) {
  case (?? p) assert (p == 9);
  case _ Prim.trap("");
};

let t1 = n1 ?? 42;
assert (t1 == 42);
let t2 = n2 ?? 42;
assert (t2 == 42);

let u1 = nat ?? Prim.trap("");
assert (u1 == 5);

let w1 = nn1 ?? ?7;
assert (w1 == null);
let w2 = nn2 ?? ?7;
assert (w2 == null);
let w3 = nn3 ?? ?7;
assert (w3 == ?1);
let w4 = nn4 ?? ?7;
assert (w4 == ?1);

let q1 = nn0 ?? n2 ?? 42;
assert (q1 == 42);

module WithDo {
  func f(n : Nat) : ?Int { ?(n + 1) };
  public func app(m : Nat) : Int {
    (do ? {
      let y = f(m)!;
      let z = f(m + 1)!;
      y + z
    }) ?? 0;
  }
};

assert (WithDo.app(1) == 5);

// Blocks
let b1 = (do { // block is not allowed on LHS
  let x = 1;
  ?{x};
}) ?? {{x=0}}; // block is allowed on RHS, so the record needs extra braces
assert (b1 == {x=1});
let br = ?{x=1};
let b2 = br ?? {
  let x = 2;
  {x}
};
assert (b2 == {x=1});

// Short-circuit: RHS must not be evaluated when LHS is Some
do {
  var counter = 0;
  func sideEffect() : Nat { counter += 1; 99 };

  let s1 = nat ?? sideEffect();
  assert (s1 == 5);
  assert (counter == 0);

  let s3 = n2 ?? sideEffect();
  assert (s3 == 99);
  assert (counter == 1);
};

// Function calls returning options.
// Parentheses are required around `?? default` because `??` binds looser than `==`.
do {
  func lookup(key : Text) : ?Nat { if (key == "a") ?1 else null };
  assert ((lookup("a") ?? 0) == 1);
  assert ((lookup("b") ?? 0) == 0);
};

// Mutable variables
do {
  var x : ?Nat = ?10;
  assert ((x ?? 0) == 10);
  x := null;
  assert ((x ?? 0) == 0);
};

// Subtyping: Nat inner with Int default yields Int
let sub1 : Int = nat ?? -1;
assert (sub1 == 5);
let sub2 : Int = n2 ?? -1;
assert (sub2 == -1);

// Chaining: a ?? b ?? c (right-associative)
let ch1 = n2 ?? nat ?? 99;
assert (ch1 == 5);
let ch2 = n1 ?? n2 ?? 42;
assert (ch2 == 42);
let ch3 = nat ?? n2 ?? 42;
assert (ch3 == 5);

// Triple-nested option: unwraps one layer
let tn1 = nn3 ?? ??0;
assert (tn1 == ??1);
let tn2 = nn0 ?? ??0;
assert (tn2 == ??0);

// Whitespace disambiguation: `a?? b` is binary `??`, since the lexer only requires
// whitespace AFTER the second `?` to emit NULLCOALESCE.
let ws1 = nat?? 0;
assert (ws1 == 5);
let ws2 = n2?? 0;
assert (ws2 == 0);
