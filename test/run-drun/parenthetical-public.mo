// Test that parenthetical annotations are accepted before `public`
persistent actor {
  (with encoder = func () : Blob = "")
  public func go() : async () {};
}

//SKIP run
//SKIP run-ir
//SKIP run-low
