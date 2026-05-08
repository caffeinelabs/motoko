// Tail-recursion-modulo-constructor: the recursive `map` over `?(<head>, <tail>)`
// gets split into a wrapper + worker pair, with the worker `return_call`-ing itself.
// See `.claude/plans/modulo-constructor.md`.

type List<T> = ?(T, List<T>);

func map<A, B>(xs : List<A>, f : A -> B) : List<B> =
  switch xs {
    case null         null;
    case (?(h, t))    ?(f(h), map<A, B>(t, f));
  };

let xs : List<Nat> = ?(1, ?(2, ?(3, null)));
let ys : List<Nat> = map<Nat, Nat>(xs, func(x) = x * 10);

// Spot-check: head 10
switch ys {
  case (?(h, _)) assert h == 10;
  case null      assert false;
};

//MOC-FLAG --experimental-tailcalls
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY

// IR interpreter doesn't yet know `StorePrim` — verified semantically via wasm-run.
//SKIP run-ir
//SKIP run-low

// Worker emitted as a sibling of the wrapper, named `$$<wrapper>'/0`.
//CHECK: func $$map'/0
// Worker reads its input from parent.1 (offset 33 = header_size 3 * 8 + slot 1 * 8 + skew 1).
//CHECK: local.get $$parent/0
//CHECK-NEXT: i64.load offset=9
//CHECK-NEXT: i64.load offset=33
// Worker tail-calls itself — the whole point of TRMC.
//CHECK: return_call $$map'/0
