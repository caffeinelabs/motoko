class Box<A>() {
  var item : ?A = null;
  public func fill(i : A) {
    item := ?i
  }
};
// Should be `let box = Box<Nat>();`
let box = Box();
box.fill(10);
