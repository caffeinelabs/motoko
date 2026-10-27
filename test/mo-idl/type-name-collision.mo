// Candid names generated for monomorphized or same-named types must not
// clash with user-written type names that look like generated ones.
actor {
  // `Gen<Text>` would be named `Gen_1`
  type Gen<T> = { x : T };
  type Gen_1 = { y : Bool };

  // the second `Dup` would be named `Dup__1`
  module A { public type Dup = { a : Nat } };
  module B { public type Dup = { b : Text } };
  type Dup__1 = { c : Bool };

  // `Rec<Text>` is reached while `Rec_1` is still being translated
  type Rec<T> = { x : T };
  type Rec_1 = { y : Rec<Text> };

  public func gen(_a : Gen<Nat>, _b : Gen<Text>, _c : Gen_1) : async () {};
  public func dup(_a : A.Dup, _b : B.Dup, _c : Dup__1) : async () {};
  public func rec(_a : Rec<Nat>, _b : Rec_1) : async () {};
};
