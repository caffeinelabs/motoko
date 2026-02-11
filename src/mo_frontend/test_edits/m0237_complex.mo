type Order = { #less; #equal; #greater };

module Nat {
  public func compare(_ : Nat, _ : Nat) : Order { #equal };
};

module Text {
  public func compare(_ : Text, _ : Text) : Order { #equal };
};

module M {
  // implicit in the middle: f(self, implicit, key)
  public func get<K, V>(
    self : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
    key : K,
  ) : ?V { ignore self; ignore key; null };

  // two adjacent implicits: f(self, implicit1, implicit2, key, value)
  public func put<K, V>(
    self : [(K, V)],
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    _cmpV : (implicit : (compare : (V, V) -> Order)),
    key : K,
    value : V,
  ) : [(K, V)] { ignore key; ignore value; self };

  // implicit at the end
  public func find<K, V>(
    self : [(K, V)],
    key : K,
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : ?V { ignore self; ignore key; null };
  public func sort1<K, V>(
    self : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : [(K, V)] { self };
  public func sort2<K, V>(
    notSelf : [(K, V)],
    _cmp : (implicit : (compare : (K, K) -> Order)),
  ) : [(K, V)] { notSelf };

  // all implicits: f(implicit1, implicit2)
  public func make<K, V>(
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    _cmpV : (implicit : (compare : (V, V) -> Order)),
  ) : [(K, V)] { [] };

  // non-adjacent implicits: f(self, implicit1, key, implicit2, value)
  public func update<K, V>(
    self : [(K, V)],
    _cmpK : (implicit : (compare : (K, K) -> Order)),
    key : K,
    _cmpV : (implicit : (compare : (V, V) -> Order)),
    value : V,
  ) : [(K, V)] { ignore key; ignore value; self };
};

// implicit in the middle
let data : [(Nat, Text)] = [];
ignore M.get(data, Nat.compare, 1);

// two adjacent implicits
ignore M.put(data, Nat.compare, Text.compare, 1, "a");

// implicit at the end
ignore M.find(data, 1, Nat.compare);
ignore data.find(1, ); // expected result after applying the edits, leaves trailing comma
ignore M.sort1(data, Nat.compare);
ignore data.sort1(); // no trailing comma here
ignore M.sort2(data, Nat.compare);
ignore M.sort2(data, ); // leaves trailing comma here

// all implicits
let _ = M.make<Nat, Text>(Nat.compare, Text.compare);

// non-adjacent implicits
ignore M.update(data, Nat.compare, 1, Text.compare, "a");

// multiline: two adjacent implicits
ignore M.put(
  data,
  Nat.compare,
  Text.compare,
  1,
  "a",
);
// multiline: with different indentation levels
ignore M.put(
  data,
   Nat.compare, // expected: edit would remove this comment
     Text.compare,
    1,
  "a",
);
