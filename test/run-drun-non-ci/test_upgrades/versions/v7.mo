import Prim "mo:⛔";

persistent actor {
  let f1 = 1;
  let f2 = 2;
  let f3 = 3;
  let f4 = 4;
  let f5 = 5;
  let f6 = 6;
  let f7 = 7;
  let f8 = 8;

  public func sum() : async Int {
    f1 + f2 + f3 + f4 + f5 + f6 + f7 + f8;
  };
};
