actor {
  public query func foo() : async () {};
}

//CALL query foo "NOTDIDL"

//SKIP run
//SKIP run-ir
//SKIP run-low
