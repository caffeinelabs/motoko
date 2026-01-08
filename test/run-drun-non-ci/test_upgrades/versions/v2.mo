import Prim "mo:⛔";

persistent actor {
  let f1 = 1;
  let f2 = 2;
  let f3 = 3;

  public func sum() : async Int {
    f1 + f2 + f3;
  };
};
