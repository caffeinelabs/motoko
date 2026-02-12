import Prim "mo:prim";

module {

  public func run(_ : {}) : {
    var zero : Nat;
    var one : [var Nat];
    var two : [var Text];
  } {
    {
      var zero : Nat = 0;
      var one : [var Nat] = [var 1, 2, 3, 4];
      var two : [var Text] = [var "1", "2", "3", "4"];
    };
  }

};
