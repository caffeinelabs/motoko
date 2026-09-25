module Map {

  public type Map<T, U> = {
    var size : Nat;
    var other : Nat
  };

  public func empty<T, U>() : Map<T, U> = {
    var size = 0;
    var other = 0
  };

  public func size<T, U>(self : Map<T,U>) : Nat { self.size };

};

let m = Map.empty<Nat, Text>();

let _n1 = m.size(); // call method
let _n2 = m.size; // read field

let _i1 = m.other; // read field
let _i2 = m.other(); // call unavailable method
