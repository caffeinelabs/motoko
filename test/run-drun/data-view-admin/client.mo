import Prim "mo:⛔";

// Use an actor class to create a third-party caller of __view query
actor class Client(server : actor { __view : query () -> async [Nat]}) {

  public func test() : async [Nat] {
     await server.__view(); // should succeed/fail depening on admin rights
  };

}
