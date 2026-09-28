actor Composites {

  func ok() {};

  // local async
  func f() : async () {};

  // local async*
  func s() : async* () {};

  // update
  public func m() : async() {};

  // oneway
  public func o() : () {};

  public composite query func cq() : async () {
    ok(); // allow local, non-async call
  };

  public query func q() : async () {
    ignore cq(); // reject composite call
  };

  public composite query func cqm() : async () {
    ignore m(); // reject update call
  };

  public composite query func cqo() : async () {
    o(); // reject oneway call
  };

  public composite query func cqf() : async () {
    ignore f(); // reject local async call
  };

  public composite query func cqs() : async () {
    ignore s(); // reject local async* call
  };

  public composite query func cqa() : async () {
    ignore async (); // reject anonymous async
  };

  public composite query func cqas() : async () {
    //ignore async* (); // odd syntax error!
    ignore async* {};
  };

  // bad update
  public func n() : async() {
    ignore cq(); // reject
  };

  // bad oneway
  public func p() : () {
    ignore cq(); // reject
    ignore async { cq() }; // reject
  };

  // bad query
  public query func r() : async () {
    ignore cq(); // reject
  };

}
