// v1 stress: 3-element spine with bullet at slot 2 (not slot 1).
// `?(Nat, T, recur)` shape exercises the data-driven setup.

type Triple<T> = ?(Nat, T, Triple<T>);

func tripleMap<A, B>(xs : Triple<A>, f : A -> B) : Triple<B> =
  switch xs {
    case null  null;
    case (?(n, h, t))  ?(n + 1, f h, tripleMap<A, B>(t, f));
  };

let xs : Triple<Nat> = ?(0, 1, ?(0, 2, null));
let ys : Triple<Nat> = tripleMap<Nat, Nat>(xs, func x = x * 10);

switch ys {
  case (?(n, h, _))  { assert n == 1; assert h == 10 };
  case null          assert false;
};

//MOC-FLAG --experimental-tailcalls
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//SKIP run-ir
//SKIP run-low

// Worker named after the wrapper.
//CHECK: func $$tripleMap'/0
// Worker reads the bullet from parent.2 (offset 41 = 3-word header * 8 + slot 2 * 8 + skew 1).
//CHECK: local.get $$parent/0
//CHECK-NEXT: i64.load offset=9
//CHECK-NEXT: i64.load offset=41
// Worker tail-calls itself.
//CHECK: return_call $$tripleMap'/0
