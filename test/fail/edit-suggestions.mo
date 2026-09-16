//MOC-FLAG -W=M0223,M0236,M0237 --all-libs --package core $MOTOKO_CORE --error-format=json
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Text "mo:core/Text";
import { type Order } "mo:core/Order";

// --- M0223: redundant type instantiation ---

do {
  func inferred<T>(x : T) : T = x;
  let n1 = inferred<Nat>(1);
  ignore n1;
};

// --- M0236: contextual dot notation ---

do {
  let m = Map.empty<Nat, Text>();
  let m2 = Map.empty<Int, Text>();

  // single arg
  ignore Map.size(m); // warn M0236

  // multi arg, no implicit in scope -> M0230 error + M0236 warn
  ignore Map.get(m2, 1); // warn M0236

  // multi arg with implicit -> M0236 + M0237
  ignore Map.get(m, Nat.compare, 1); // warn M0236 + M0237

  // complex receiver
  ignore Map.size(
    Map.empty<Nat, Text>()
  ); // warn M0236

  // multiline call -> M0236 + M0237
  Map.add(
    m,
    Nat.compare,
    1,
    "John",
  ); // warn M0236 + M0237
};

// --- M0237: implicit argument removal ---

do {
  let m = Map.empty<Nat, Text>();

  // single line
  ignore m.get(Nat.compare, 1); // warn M0237

  // multiline
  ignore m.get(
    Nat.compare,
    1,
  ); // warn M0237
};

// --- M0237: complex implicit patterns ---

module Impl {
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

do {
  let data : [(Nat, Text)] = [];

  // implicit in the middle -> M0237 only: the array's built-in `get` field would shadow `data.get(...)`
  ignore Impl.get(data, Nat.compare, 1);

  // two adjacent implicits -> M0236 + M0237 x2
  ignore Impl.put(data, Nat.compare, Text.compare, 1, "a");

  // implicit at the end -> M0236 + M0237
  ignore Impl.find(data, 1, Nat.compare);
  ignore Impl.sort1(data, Nat.compare); // -> M0236 + M0237
  ignore Impl.sort2(data, Nat.compare); // no dot suggestion (notSelf), M0237 only

  // all implicits -> M0237 x2
  let _ = Impl.make<Nat, Text>(Nat.compare, Text.compare);

  // non-adjacent implicits -> M0236 + M0237 x2
  ignore Impl.update(data, Nat.compare, 1, Text.compare, "a");

  // multiline: two adjacent implicits -> M0236 + M0237 x2
  ignore Impl.put(
    data,
    Nat.compare,
    Text.compare,
    1,
    "a",
  );
};

// --- Mix: M0223 + M0236 + M0237 ---

do {
  // NB: Must use `let _ = ...` to get the 'redundant type instantiation' error
  let _ = Map.add<Nat, Text>(
    Map.empty<Nat, Text>(),
    Nat.compare,
    1,
    "John",
  ); // warn M0223 + M0236 + M0237
};

// --- M0236: regression — unparenthesized single-arg call (#6096) ---

do {
  let m = Map.empty<Nat, Text>();

  // single arg, no parens
  ignore Map.size m; // warn M0236

  // single arg, no parens, with type instantiation
  ignore Map.size<Nat, Text> m; // warn M0236
};

// --- M0236: non-postfix receiver -> no warning (no clean autofix) ---

do {
  // BinE receiver: `Nat.toText((x * 6364136223846793005 + 1442695040888963407) % 4294967296)`
  // would rewrite to `(...).toText()` adding outer parens; suppress instead.
  let pos = 1;
  ignore Nat.toText((pos * 6364136223846793005 + 1442695040888963407) % 4294967296); // no-warn

  // IfE receiver — likewise.
  ignore Nat.toText(if (pos == 0) 1 else 2); // no-warn
};

// Suggest context dot conflicts with field resolution
do {
  module Tree {
    public func delete<K, V>(self : RBTree<K, V>, key : K) : Bool { ignore self; ignore key; true };
    public func size<K, V>(self : RBTree<K, V>) : Nat { ignore self; 1 };
  };

  type RBTree<K, V> = { delete : K -> (); size : () -> Nat };
  func make<K, V>() : RBTree<K, V> {
    { delete = func(k : K) { ignore k }; size = func() : Nat { 1 } }
  };
  let t = make<Nat, Text>();
  let n1 = Tree.size(t); // no warn because .size does not resolve to Tree.size!
  let n2 = t.size();
  let b = Tree.delete(t, 0); // no warn, as above
  t.delete(0);
  ignore (n1, n2, b);
};

// Field shadowing decides the suggestion exactly like real dot resolution
do {
  module Counter {
    public func count(self : Cnt) : Nat { self.count + 1 };
    public func reset(self : Cnt) : Cnt { ignore self; { count = 0; reset = self.reset } };
  };
  type Cnt = { count : Nat; reset : () -> Cnt };
  func mk() : Cnt { { count = 1; reset = func() : Cnt { mk() } } };
  let cnt = mk();
  // warn M0236 — the non-function `count` field does not shadow, `cnt.count()` still resolves to Counter.count
  ignore Counter.count(cnt);
  // no warn — the function-typed `reset` field shadows, `cnt.reset()` would call the field instead of Counter.reset
  ignore Counter.reset(cnt);
};

// Built-in pseudo-fields of blobs and text shadow like record fields
do {
  module Str {
    public func size(self : Text) : Nat { self.size() };
    public func trim2(self : Text) : Text { self };
  };
  module Bytes {
    public func get(self : Blob, i : Nat) : Nat8 { self.get(i) };
    public func rank(self : Blob) : Nat { ignore self; 0 };
  };
  let t = "hi";
  let b : Blob = "\00\01";
  ignore Str.size(t); // no warn — text's built-in `size` shadows
  ignore Str.trim2(t); // warn M0236
  ignore Bytes.get(b, 0); // no warn — blob's built-in `get` shadows
  ignore Bytes.rank(b); // warn M0236
};

// Phantom type parameter: probing with the promoted receiver still resolves, `s.size()` compiles
do {
  module Phantom {
    public type Box<K> = { arr : [Nat] };
    public func size<K>(self : Box<K>) : Nat { self.arr.size() };
  };
  let s : Phantom.Box<Text> = { arr = [] };
  ignore Phantom.size(s); // warn M0236
};
