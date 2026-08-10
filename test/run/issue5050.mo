import Prim "mo:⛔";

// `class` in expression position must yield its constructor, not `()`.
type Iter = { next : () -> ?Nat };
func vals(xs : [Nat]) : () -> Iter = class() : Iter {
  var i = 0;
  public func next() : ?Nat { if (i >= xs.size()) null else { let j = i; i += 1; ?xs[j] } };
};
let it = vals([7, 8])();
Prim.debugPrint(debug_show (it.next(), it.next(), it.next()));

// Same defect via a `do` block. The decoys populate the function table, so a
// unit-typed `mk` calls whatever sits at index 0 instead of the constructor.
type Box = { x : Nat };
func f0() : Box { { x = 700 } };
func f1() : Box { { x = 701 } };
let decoys : [() -> Box] = [f0, f1];
let mk = do { class C() { public let x = 42 } };
Prim.debugPrint(debug_show (mk().x, decoys[1]().x));
