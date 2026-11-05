import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A table built with indexes keeps one index per field list declared, an index declared twice
// being one index. Its counts are right from the first row on, and stay right through inserts,
// replacements and removals. Walks forward and backward, bounded and unbounded, hand back every
// row they cover exactly once, in key order, across batches of 64 rows; minEntry and maxEntry
// skip null keys, and are null when every key is null. A read names an index by the field list
// the table was built with or by any other list of the same fields. Index keys are checked against
// the index's fields, and a read through fields the table has no index on traps. maintain has no
// work on an empty table
actor {
  type Person = { name : Text; city : Text; age : Nat; nick : ?Text };

  let city = ["city"];
  let pair = ["age", "city"];
  let nick = ["nick"];

  let people = NativeTable.empty<Nat, Person>().createIndex(city).createIndex(pair).createIndex(["city"]).createIndex(nick).build();
  let small = NativeTable.empty<Nat, { b : Nat8; c : Char }>().createIndex(["b"]).createIndex(["c"]).build();
  let opt = NativeTable.empty<Nat, { v : ?Nat }>().createIndex(["v"]).build();
  let long = NativeTable.empty<Nat, { t : Text }>().createIndex(["t"]).build();
  let fresh = NativeTable.empty<Nat, Person>().build();

  let n = 2_000;
  let cities = 20;
  let ages = 50;
  let nicks = 7;

  func person(k : Nat) : Person = {
    name = "person " # debug_show k;
    city = "city " # debug_show (k % cities);
    age = k % ages;
    nick = if k % 3 == 0 { null } else { ?("n" # debug_show (k % nicks)) }
  };

  // The rows the table should hold, by key
  let model = Prim.Array_init<?Person>(n + 100, null);

  func cityName(c : Nat) : Text = "city " # debug_show c;

  // The number of counts of `people` that differ from the rows of the model, over every city,
  // every age and city, and every nick, null included
  func wrong() : Nat {
    let byCity = Prim.Array_init<Nat>(cities + 1, 0);
    let byPair = Prim.Array_init<Nat>(ages * cities, 0);
    let byNick = Prim.Array_init<Nat>(nicks + 1, 0);
    for r in model.values() {
      switch r {
        case null {}
        case ?p {
          if p.city == "moved" { byCity[cities] += 1 } else {
            byCity[textCity(p.city)] += 1;
            byPair[p.age * cities + textCity(p.city)] += 1
          };
          switch p.nick {
            case null { byNick[nicks] += 1 }
            case ?t { byNick[textNick(t)] += 1 }
          }
        }
      }
    };
    var wrong = 0;
    var c = 0;
    while c < cities {
      if people.count(city, [#text(cityName(c))]) != byCity[c] { wrong += 1 };
      c += 1
    };
    if people.count(city, [#text "moved"]) != byCity[cities] { wrong += 1 };
    var a = 0;
    while a < ages {
      c := 0;
      while c < cities {
        if people.count(pair, [#nat a, #text(cityName(c))]) != byPair[a * cities + c] { wrong += 1 };
        c += 1
      };
      a += 1
    };
    var i = 0;
    while i < nicks {
      if people.count(nick, [#text("n" # debug_show i)]) != byNick[i] { wrong += 1 };
      i += 1
    };
    if people.count(nick, [#null_]) != byNick[nicks] { wrong += 1 };
    wrong
  };

  func textCity(t : Text) : Nat {
    var c = 0;
    while c < cities {
      if t == cityName(c) { return c };
      c += 1
    };
    Prim.trap("no such city")
  };

  func textNick(t : Text) : Nat {
    var i = 0;
    while i < nicks {
      if t == "n" # debug_show i { return i };
      i += 1
    };
    Prim.trap("no such nick")
  };

  public func fill() : async () {
    var k = 0;
    while k < n {
      people.add(k, person(k));
      model[k] := ?person(k);
      k += 1
    };
    Prim.debugPrint(debug_show (people.size(), people.indexes))
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show { wrong = wrong() });
    Prim.debugPrint(debug_show (people.count(city, [#text "city 3"]), people.count(pair, [#nat 3, #text "city 3"]), people.count(nick, [#null_])));
    // The same indexes, named by other lists of the same fields
    let cityAgain = Prim.Array_tabulate<Text>(1, func _ = "ci" # "ty");
    let pairAgain = ["age", "ci" # "ty"];
    Prim.debugPrint(debug_show (people.count(cityAgain, [#text "city 3"]), people.count(pairAgain, [#nat 3, #text "city 3"]), people.count(["nick"], [#null_])))
  };

  // Removals, replacements that change a row's keys, and inserts
  public func change() : async () {
    var k = 0;
    while k < n {
      if k % 5 == 2 {
        people.remove(k);
        model[k] := null
      } else if k % 7 == 0 {
        let p = { person(k) with city = "moved"; nick = ?"n1" };
        people.add(k, p);
        model[k] := ?p
      };
      k += 1
    };
    while k < n + 100 {
      people.add(k, person(k));
      model[k] := ?person(k);
      k += 1
    };
    Prim.debugPrint(debug_show people.size())
  };

  type Walk = { next : () -> ?(Nat, Person) };

  // The number of rows `walk` hands back, and of the things wrong with it: a row that is not the
  // model's row of its key, a key that comes twice, a row `ordered` does not take after the row
  // before it, a row `wanted` does not take, and a walk that does not hand back as many rows as
  // the model has rows `wanted` takes
  func walked(walk : Walk, ordered : (Person, Person) -> Bool, wanted : Person -> Bool) : { rows : Nat; wrong : Nat } {
    let seen = Prim.Array_init<Bool>(n + 100, false);
    var rows = 0;
    var wrong = 0;
    var last : ?Person = null;
    for (k, p) in walk {
      rows += 1;
      if seen[k] or model[k] != ?p or not wanted(p) { wrong += 1 };
      seen[k] := true;
      switch last {
        case ?q { if not ordered(q, p) { wrong += 1 } }
        case null {}
      };
      last := ?p
    };
    var want = 0;
    for r in model.values() {
      switch r {
        case ?p { if wanted(p) { want += 1 } }
        case null {}
      }
    };
    if rows != want { wrong += 1 };
    { rows; wrong }
  };

  func cityUp(a : Person, b : Person) : Bool = a.city <= b.city;
  func cityDown(a : Person, b : Person) : Bool = a.city >= b.city;
  func pairUp(a : Person, b : Person) : Bool = a.age < b.age or (a.age == b.age and a.city <= b.city);
  func pairDown(a : Person, b : Person) : Bool = pairUp(b, a);
  // Null comes before every nick
  func nickUp(a : Person, b : Person) : Bool = switch (a.nick, b.nick) {
    case (null, _) { true }
    case (?_, null) { false }
    case (?x, ?y) { x <= y }
  };
  func nickDown(a : Person, b : Person) : Bool = nickUp(b, a);
  func any(_ : Person) : Bool = true;

  public query func walks() : async () {
    Prim.debugPrint(debug_show {
      all = walked(people.indexEntries(city, null, null), cityUp, any);
      allBack = walked(people.reverseIndexEntries(city, null, null), cityDown, any);
      some = walked(people.indexEntries(city, ?[#text "city 10"], ?[#text "city 15"]), cityUp, func p = p.city >= "city 10" and p.city <= "city 15");
      someBack = walked(people.reverseIndexEntries(city, ?[#text "city 15"], ?[#text "city 10"]), cityDown, func p = p.city >= "city 10" and p.city <= "city 15");
      one = walked(people.indexEntries(city, ?[#text "city 3"], ?[#text "city 3"]), cityUp, func p = p.city == "city 3");
      oneBack = walked(people.reverseIndexEntries(city, ?[#text "city 3"], ?[#text "city 3"]), cityDown, func p = p.city == "city 3");
      from = walked(people.indexEntries(city, ?[#text "city 5"], null), cityUp, func p = p.city >= "city 5");
      fromBack = walked(people.reverseIndexEntries(city, ?[#text "city 5"], null), cityDown, func p = p.city <= "city 5");
      to = walked(people.indexEntries(city, null, ?[#text "city 12"]), cityUp, func p = p.city <= "city 12");
      toBack = walked(people.reverseIndexEntries(city, null, ?[#text "city 12"]), cityDown, func p = p.city >= "city 12");
      between = walked(people.indexEntries(city, ?[#text "city 15a"], ?[#text "city 17"]), cityUp, func p = p.city > "city 15a" and p.city <= "city 17");
      crossed = walked(people.indexEntries(city, ?[#text "city 9"], ?[#text "city 1"]), cityUp, func _ = false);
      pairs = walked(people.indexEntries(pair, ?[#nat 3, #text "city 3"], ?[#nat 7, #text "city 7"]), pairUp, func p = pairUp({ p with age = 3; city = "city 3" }, p) and pairUp(p, { p with age = 7; city = "city 7" }));
      pairsBack = walked(people.reverseIndexEntries(pair, ?[#nat 7, #text "city 7"], ?[#nat 3, #text "city 3"]), pairDown, func p = pairUp({ p with age = 3; city = "city 3" }, p) and pairUp(p, { p with age = 7; city = "city 7" }));
      nicks = walked(people.indexEntries(nick, null, null), nickUp, any);
      nicksBack = walked(people.reverseIndexEntries(nick, null, null), nickDown, any);
      nulls = walked(people.indexEntries(nick, ?[#null_], ?[#null_]), nickUp, func p = p.nick == null)
    })
  };

  // The ends of an index: the field `shown` shows of the row, whether the row is the model's row
  // of its key, and whether it is the first row of the walk from its key
  func end(e : ?(Nat, Person), shown : Person -> Text, walk : Text -> Walk) : ?(Text, Bool, Bool) {
    switch e {
      case null { null }
      case ?(k, p) { ?(shown(p), model[k] == ?p, walk(shown(p)).next() == ?(k, p)) }
    }
  };

  func nickOf(p : Person) : Text = switch p.nick { case ?t { t } case null { "" } };

  public query func ends() : async () {
    Prim.debugPrint(debug_show {
      minCity = end(people.minEntry(city), func p = p.city, func t = people.indexEntries(city, ?[#text t], null));
      maxCity = end(people.maxEntry(city), func p = p.city, func t = people.reverseIndexEntries(city, ?[#text t], null));
      minNick = end(people.minEntry(nick), nickOf, func t = people.indexEntries(nick, ?[#text t], null));
      maxNick = end(people.maxEntry(nick), nickOf, func t = people.reverseIndexEntries(nick, ?[#text t], null))
    })
  };

  // An index whose rows all have a null key has no ends; an empty one has no rows either
  public func nullEnds() : async () {
    let v = ["v"];
    func rows(walk : { next : () -> ?(Nat, { v : ?Nat }) }) : Nat {
      var n = 0;
      for _ in walk { n += 1 };
      n
    };
    Prim.debugPrint(debug_show (opt.minEntry(v), opt.maxEntry(v), rows(opt.indexEntries(v, null, null)), rows(opt.reverseIndexEntries(v, null, null))));
    var k = 0;
    while k < 100 {
      opt.add(k, { v = null });
      k += 1
    };
    Prim.debugPrint(debug_show (opt.minEntry(v), opt.maxEntry(v), rows(opt.indexEntries(v, null, null)), rows(opt.reverseIndexEntries(v, null, null))));
    opt.add(100, { v = ?9 });
    opt.add(101, { v = ?5 });
    opt.add(102, { v = ?9 });
    Prim.debugPrint(debug_show (opt.minEntry(v), opt.maxEntry(v), rows(opt.indexEntries(v, ?[#nat 5], null)), rows(opt.reverseIndexEntries(v, null, ?[#nat 5]))))
  };

  let longs = 150;

  // Distinct Texts of 101 to 3,083 bytes, so that their index keys do not fit the buffers a walk
  // starts with
  func longText(k : Nat) : Text {
    var t = debug_show ((k * 7) % longs);
    var i = 0;
    while i < 100 + 20 * ((k * 11) % longs) {
      t #= "x";
      i += 1
    };
    t
  };

  // Walks and ends of an index on long Texts
  public func longKeys() : async () {
    let texts = Prim.Array_tabulate<Text>(longs, longText);
    let t = ["t"];
    var k = 0;
    while k < longs {
      long.add(k, { t = texts[k] });
      k += 1
    };
    func walked(walk : { next : () -> ?(Nat, { t : Text }) }, up : Bool, wanted : Text -> Bool) : { rows : Nat; wrong : Nat } {
      let seen = Prim.Array_init<Bool>(longs, false);
      var rows = 0;
      var wrong = 0;
      var last : ?Text = null;
      for (k, r) in walk {
        rows += 1;
        if seen[k] or r.t != texts[k] or not wanted(r.t) { wrong += 1 };
        seen[k] := true;
        switch last {
          case ?l { if (if up { l > r.t } else { l < r.t }) { wrong += 1 } }
          case null {}
        };
        last := ?r.t
      };
      var want = 0;
      for x in texts.values() { if wanted(x) { want += 1 } };
      if rows != want { wrong += 1 };
      { rows; wrong }
    };
    var min = 0;
    var max = 0;
    k := 0;
    while k < longs {
      if texts[k] < texts[min] { min := k };
      if texts[k] > texts[max] { max := k };
      k += 1
    };
    let (lo, hi) = if texts[10] <= texts[100] { (texts[10], texts[100]) } else { (texts[100], texts[10]) };
    Prim.debugPrint(debug_show {
      all = walked(long.indexEntries(t, null, null), true, func _ = true);
      allBack = walked(long.reverseIndexEntries(t, null, null), false, func _ = true);
      some = walked(long.indexEntries(t, ?[#text lo], ?[#text hi]), true, func x = x >= lo and x <= hi);
      someBack = walked(long.reverseIndexEntries(t, ?[#text hi], ?[#text lo]), false, func x = x >= lo and x <= hi);
      ends = (long.minEntry(t) == ?(min, { t = texts[min] }), long.maxEntry(t) == ?(max, { t = texts[max] }))
    })
  };

  public func ranges() : async () {
    small.add(1, { b : Nat8 = 7; c = 'A' });
    small.add(2, { b : Nat8 = 255; c = 'A' });
    Prim.debugPrint(debug_show (small.count(["b"], [#int 7]), small.count(["b"], [#nat 255]), small.count(["c"], [#nat 65])))
  };

  // An empty table has no work pending
  public func upkeep() : async () {
    Prim.debugPrint(debug_show (fresh.maintain(0), fresh.maintain(1_000_000)))
  };

  public func unknownField() : async () {
    ignore NativeTable.empty<Nat, Person>().createIndex(["nope"]).build()
  };
  public func noFields() : async () { ignore NativeTable.empty<Nat, Person>().createIndex([]).build() };
  // The fields of an index in another order are another index
  public func noIndex() : async () { ignore people.count(["city", "age"], [#text "city 3", #nat 3]) };
  public func wrongKind() : async () { ignore people.count(city, [#nat 3]) };
  public func wrongLength() : async () { ignore people.count(pair, [#nat 3]) };
  public func nullKey() : async () { ignore people.count(city, [#null_]) };
  public func tooLarge() : async () { ignore small.count(["b"], [#nat 256]) };
  public func negative() : async () { ignore small.count(["b"], [#int(-1)]) };
  public func surrogate() : async () { ignore small.count(["c"], [#nat 0xD800]) };
  public func walkWrongKind() : async () { ignore people.indexEntries(city, ?[#nat 3], null) };
  public func walkWrongLength() : async () { ignore people.reverseIndexEntries(pair, null, ?[#nat 3]) };
  public func walkNoIndex() : async () { ignore people.indexEntries(["age"], null, null) };
  public func endNoIndex() : async () { ignore people.maxEntry(["name"]) }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress fill "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress walks "DIDL\x00\x00"
//CALL ingress ends "DIDL\x00\x00"
//CALL ingress nullEnds "DIDL\x00\x00"
//CALL ingress longKeys "DIDL\x00\x00"
//CALL ingress ranges "DIDL\x00\x00"
//CALL ingress upkeep "DIDL\x00\x00"
//CALL ingress unknownField "DIDL\x00\x00"
//CALL ingress noFields "DIDL\x00\x00"
//CALL ingress noIndex "DIDL\x00\x00"
//CALL ingress wrongKind "DIDL\x00\x00"
//CALL ingress wrongLength "DIDL\x00\x00"
//CALL ingress nullKey "DIDL\x00\x00"
//CALL ingress tooLarge "DIDL\x00\x00"
//CALL ingress negative "DIDL\x00\x00"
//CALL ingress surrogate "DIDL\x00\x00"
//CALL ingress walkWrongKind "DIDL\x00\x00"
//CALL ingress walkWrongLength "DIDL\x00\x00"
//CALL ingress walkNoIndex "DIDL\x00\x00"
//CALL ingress endNoIndex "DIDL\x00\x00"
