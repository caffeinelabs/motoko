//MOC-FLAG -A=M0194
//MOC-FLAG -W M0236
// test suggestion of contextual dot notation,
// excluding binary equals and compare(XXX)
type Order = {
  #less; #equal; #greater;
};

module Any {
//  public func compare(n : Any, m : Any) : Order { #equal };
};

module Nat {
  public func compare(_self : Nat, _m : Nat) : Order { #equal };
  public func compareBy(_self : Nat, _m : Nat) : Order { #equal };
  public func equal(_self : Nat, _m : Nat) : Bool { true };
};

module Text {
  public func compare(_self : Text, _m : Text) : Order { #equal };
  public func equal(_self : Text, _m : Text) : Bool { true };
};

module Odd {
  public type Self = {#odd};
  public func compare(self : Self, _m : Self, _o : Self) : Order { #equal };
  public func equal(self : Self) : Bool { true };
};

module Amb1 {
  public type Self = {#amb};
  public func method(_self : Self) {};
};

module Amb2 {
  public type Self = {#amb};
  public func method(_self : Self) {}; // ambiguous with Amb1.method();
};


module Map {
  public type Map<K,V> = {map : [(K, [var V])]};
  public func empty<K, V>() : Map<K,V> = { map= []};

  public func get<K, V>(
    self : Map<K, V>,
    _compare: (implicit : (compare : (K, K) -> Order)),
    _n : K)
  : ?V {
    null
  };

  public func set<K, V>(
    self : Map<K, V>,
    _compare: (implicit : (compare : (K, K) -> Order)),
    _n : K,
    _v : V)
  : Map<K, V> {
    self
  };

  public func clone<K, V>(self : Map<K, V>) : Map<K,V> { self };

  public func size<K, V>(self : Map<K, V>) : Nat { 0 };

  public func singleton<K, V>(k : K, v : V) : Map<K, V> {
    { map = [(k, [var v])] }
  };

};

persistent actor {
  let peopleMap = Map.empty<Nat, Text>();

  ignore Nat.compare(0, 1); // no-warn
  ignore Nat.compareBy(0, 1); // no-warn
  ignore Nat.equal(0, 1); // no-warn

  ignore Text.compare("", ""); // no-warn
  ignore Text.equal("", ""); // no-warn

  ignore Odd.equal(#odd); // warn non-binary
  ignore Odd.compare(#odd, #odd, #odd); // warn non-binary

  // get
  ignore Map.get(peopleMap, Nat.compare, 1); // warn
  ignore Map.get(peopleMap, 1); // warn
  ignore Map.size(peopleMap); // warn

  ignore Map.get(Map.clone(peopleMap), Nat.compare, 1); // warn x 2

  ignore peopleMap.get(Nat.compare, 1); // ok
  ignore peopleMap.get(1); // ok

  ignore Map.singleton(1,"hello"); // ok - don't warn (no appropriate receiver)

  Amb1.method(#amb); // don't suggest, ambiguous

}
