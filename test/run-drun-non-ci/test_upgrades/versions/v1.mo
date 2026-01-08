import Prim "mo:⛔";

persistent actor {
  let f1 = 1;
  let f2 = 2;

  public func sum() : async Int {
    f1 + f2;
  };
};
