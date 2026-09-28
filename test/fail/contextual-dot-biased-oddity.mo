module Box {
  type Box<T> = {
    value : T;
  };
  public func Box<T>(t : T) : Box<T> = { value = t };
  public func value<T>(self : Box<T>) : T { self.value };
};

let b1 = Box.Box<Nat>(1);
let n1 = b1.value; // accept, field ref
let n2 = b1.value(); // accept, method ref

let b2 = Box.Box<Nat->Nat>(func x = x);
let f1 = b2.value; // accept, field ref
// both f2 and f3 are perhaps unexpected
let f2 = b2.value(1); // accept, field ref (applies the field of type Nat->Nat to 1)
let f3 = b2.value(); // reject, field ref (applies the field of type Nat->Nat to ())
