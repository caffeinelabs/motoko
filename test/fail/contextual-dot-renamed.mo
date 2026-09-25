// test contextual dot works with renamed types
module Map {

  public type Map<T, U> = {
    var size : Nat;
  };

  public func empty<T, U>() : Map<T, U> = {
    var size = 0;
  };

  public func size<T, U>(self : Map<T,U>) : Nat { self.size };

};

type Renamed<U, T> = Map.Map<T, U>;

let m : Renamed<Text, Nat> = Map.empty<Nat, Text>();

let _n1 = m.size(); // call method
let _n2 = m.size; // read field
