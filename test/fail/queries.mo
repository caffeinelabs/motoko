import Prim "mo:⛔";

actor a {

  public shared func oneway() : () { };

  public shared func request() : async () { await okquery()};

  public shared query func okquery() : async () {
  };

  public shared query func badquery() : async () {
    await (loop {}); // can't await
  };

  public shared query func badquery1() : async () {
    oneway(); // can't call a (oneway) shared method
  };

  public shared query func badquery2() : async () {
    ignore (request()); // can't call a shared method
    await request(); // can't wait its result
  };

  public shared query func badquery3() : async () {
    async { }; // can't enter an async expression
  };

  public shared query func badquery4() : async () {
    throw (Prim.error ""); // can throw
  };

  public shared query func badquery5() : async () {
    try ()  catch _  {}; // can catch
  };

};

