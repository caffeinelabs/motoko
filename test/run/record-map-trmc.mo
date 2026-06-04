// v1 stretch goal: TRMC over a record-shaped recursion.
// Currently the recogniser only matches `OptPrim (TupPrim …)`, so TRMC
// does NOT fire on this shape — the program compiles via the ordinary
// (non-tail) recursion. This test pins down the source/runtime
// behaviour; FileCheck assertions are aspirational and commented out
// until the recogniser learns the `OptPrim (BlockE … (NewObjE …))`
// shape and `StorePrim` codegen learns the `DotPrim` load shape.

type RecordList<T> = ?{ head : T; tail : RecordList<T> };

func recordMap<A, B>(xs : RecordList<A>, f : A -> B) : RecordList<B> =
  switch xs {
    case null  null;
    case (?{head; tail})  ?{ head = f head; tail = recordMap<A, B>(tail, f) };
  };

let xs : RecordList<Nat> = ?{ head = 1; tail = ?{ head = 2; tail = null } };
let ys : RecordList<Nat> = recordMap<Nat, Nat>(xs, func x = x * 10);

switch ys {
  case (?{head})  assert head == 10;
  case null       assert false;
};

//MOC-FLAG --experimental-tailcalls
//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY
//SKIP run-ir
//SKIP run-low

// (Aspirational — see comment above. Uncomment when recogniser supports NewObjE.)
// CHECK: func $$recordMap'/0
// CHECK: return_call $$recordMap'/0
