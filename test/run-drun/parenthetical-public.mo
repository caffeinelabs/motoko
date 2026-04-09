// Test that parenthetical annotations are accepted before `public`
persistent actor {
  (with encoder = func () : Blob = "")
  public func go() : async () {};
}

//CALL ingress go 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low
