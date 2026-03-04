//MOC-FLAG -A=M0194
actor {
 type Result<Ok,Err> = {
  #ok:Ok;
  #err:Err;
 };
 public func f(x:?Nat):async Result<Nat,Text> {
  switch x {
    case (? x) {#ok x};
    case null {#err "error"};
  }
 };
 public func g(x:Result<Nat,Text>):async Result<Int,()> {
  #ok(1);
 };
}
