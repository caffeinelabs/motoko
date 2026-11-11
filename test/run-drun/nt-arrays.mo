import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";
import NativeTableCells "mo:native-table/NativeTableCells";

// A row may hold arrays: of scalars, of options of scalars and of records of them, and options of
// arrays. A table with array fields builds, evolve adds an array field as an option to a table with
// rows, and a view may name an array field. fieldTypes lists an array field once, with its
// elements' scalar type, and a field of an array of records once per field of the record, as
// "field.sub". An evolve that changes an array's element type traps and changes nothing.
// Arrays of every scalar type, at the ends of each type's range, empty arrays and options of
// arrays read back equal to what was written: by get, scans, index walks and views, in packed
// rows and rows of the open group, an array far past a row's inline limit too, in a table whose
// row type changed, and after update sets an array field or another field of the row. An index,
// a filter, a total and a cell scan on an array field trap
actor {
  type Person = {
    name : Text;
    codes : [Text];
    scores : [?Nat];
    tags : ?[Text];
    history : [{ amount : Nat; since : Int; note : ?Text }]
  };

  let people = NativeTable.empty<Nat, Person>().createIndex(["name"]).build();
  let names : NativeTable.NativeTable<Nat, { name : Text; codes : [Text] }> = people.view();

  type Kinds = {
    bools : [Bool];
    n8 : [Nat8];
    n16 : [Nat16];
    n32 : [Nat32];
    n64 : [Nat64];
    i8 : [Int8];
    i16 : [Int16];
    i32 : [Int32];
    i64 : [Int64];
    nats : [Nat];
    ints : [Int];
    floats : [Float];
    chars : [Char];
    texts : [Text];
    blobs : [Blob];
    principals : [Principal];
    options : [?Int];
    records : [{ small : Nat8; text : ?Text; float : Float }];
    maybe : ?[Nat]
  };

  let kinds = NativeTable.empty<Text, Kinds>().build();

  let ends : Kinds = {
    bools = [true, false];
    n8 = [0, 255];
    n16 = [0, 65_535];
    n32 = [0, 4_294_967_295];
    n64 = [0, 18_446_744_073_709_551_615];
    i8 = [-128, 127];
    i16 = [-32_768, 32_767];
    i32 = [-2_147_483_648, 2_147_483_647];
    i64 = [-9_223_372_036_854_775_808, 9_223_372_036_854_775_807];
    nats = [0, 127, 128, 4_611_686_018_427_387_903, 4_611_686_018_427_387_904, 18_446_744_073_709_551_616, 340_282_366_920_938_463_463_374_607_431_768_211_456];
    ints = [0, -1, -64, 64, -65, -4_611_686_018_427_387_905, 9_223_372_036_854_775_808, -340_282_366_920_938_463_463_374_607_431_768_211_457];
    floats = [0.0, -0.5, 1.25e15, -3e-3];
    chars = ['a', 'ü', '\u{10FFFF}'];
    texts = ["", "Zürich", "a\u{0}b"];
    blobs = ["", "\00\FF\7F"];
    principals = [Prim.principalOfBlob(""), Prim.principalOfBlob("\04"), Prim.principalOfBlob("\01\02\03\04\05\06\07\08\09\0A\0B\0C\0D\0E\0F\10\11\12\13\14\15\16\17\18\19\1A\1B\1C\1D")];
    options = [null, ?0, ?(-1), null];
    records = [{ small = 1; text = null; float = 2.5 }, { small = 255; text = ?"x"; float = -0.0 }];
    maybe = ?[]
  };

  let empties : Kinds = {
    bools = [];
    n8 = [];
    n16 = [];
    n32 = [];
    n64 = [];
    i8 = [];
    i16 = [];
    i32 = [];
    i64 = [];
    nats = [];
    ints = [];
    floats = [];
    chars = [];
    texts = [];
    blobs = [];
    principals = [];
    options = [];
    records = [];
    maybe = null
  };

  type Old = { name : Text; age : Nat };
  type New = { name : Text; age : Nat; codes : ?[Text]; history : ?[{ amount : Nat }] };
  let old = NativeTable.empty<Nat, Old>().build();
  var changed : ?NativeTable.NativeTable<Nat, New> = null;

  // 10,000 codes: an array far past a row's inline limit, which a read gets in pieces
  func longCodes() : [Text] = Prim.Array_tabulate<Text>(10_000, func(i) = "code-" # debug_show i);

  func person(i : Nat) : Person {
    {
      name = "p" # debug_show i;
      codes = if i == 7 or i == 4_150 { longCodes() } else {
        Prim.Array_tabulate<Text>(i % 4, func(j) = "c" # debug_show (i + j))
      };
      scores = [?i, null, ?(i * 1_000_000_000_000_000_000_000)];
      tags = if i % 3 == 0 { null } else { ?["t" # debug_show i] };
      history = Prim.Array_tabulate<{ amount : Nat; since : Int; note : ?Text }>(
        i % 3,
        func(j) = { amount = i + j; since = -i; note = if j == 0 { null } else { ?"raise" } }
      )
    }
  };

  // The rows of the people table as `put` writes them and `update` changes them
  func expected(i : Nat) : Person {
    switch i {
      case 5 { { person(5) with codes = ["new"]; scores = []; tags = null } }
      case 7 { { person(7) with name = "renamed" } }
      case 4_150 { { person(4_150) with name = "renamed too"; tags = ?["x", "y"] } }
      case _ { person(i) }
    }
  };

  public func build() : async () {
    Prim.debugPrint(debug_show (people.size(), people.isEmpty(), names.size(), people.indexes));
    for (name, t) in people.fieldTypes().values() {
      Prim.debugPrint(debug_show (name, t))
    }
  };

  public func kindsRoundTrip() : async () {
    kinds.add("ends", ends);
    kinds.add("empties", empties);
    Prim.debugPrint(debug_show (kinds.get("ends") == ?ends, kinds.get("empties") == ?empties));
    Prim.debugPrint(debug_show kinds.get("ends"));
    for (k, r) in kinds.entries() {
      Prim.debugPrint(debug_show (k, r == (if k == "ends" { ends } else { empties })))
    }
  };

  // 4,200 rows: rows 0 to 4,095 fill a group, which is packed; the rest stay in the open group
  public func put() : async () {
    var i = 0;
    while i < 4_200 {
      people.add(i, person(i));
      i += 1
    };
    Prim.debugPrint(debug_show (people.size(), people.get(1), names.get(2), people.get(4_199)))
  };

  // Every row read back by get, through the view, by a scan, by keys and by an index walk
  public func check() : async () {
    var got = 0;
    var viewed = 0;
    var scanned = 0;
    var keys = 0;
    var i = 0;
    while i < 4_200 {
      let r = expected(i);
      if people.get(i) == ?r { got += 1 };
      let named = { name = r.name; codes = r.codes };
      if names.get(i) == ?named { viewed += 1 };
      i += 1
    };
    for (k, r) in people.entries() {
      if r == expected(k) { scanned += 1 }
    };
    for (k, r) in names.entries() {
      let named = { name = expected(k).name; codes = expected(k).codes };
      if r == named { scanned += 1 }
    };
    for _ in people.keys() { keys += 1 };
    let walk = people.indexEntries(["name"], ?[#text "renamed"], null);
    let walked = (walk.next() == ?(7, expected(7)), walk.next() == ?(4_150, expected(4_150)));
    let cells = people.cells(true, ["name"], null).next();
    Prim.debugPrint(debug_show { got; viewed; scanned; keys; walked; cells })
  };

  // update sets an array field, and keeps an array the update does not name, a long one too, in a
  // packed row (5, 7) and in a row of the open group (4,150)
  public func update() : async () {
    let a = people.update(5, { codes = ["new"]; scores = [] : [?Nat]; tags = null : ?[Text] });
    let b = people.update(7, { name = "renamed" });
    let c = people.update(4_150, { name = "renamed too"; tags = ?["x", "y"] });
    Prim.debugPrint(debug_show (a, b, c, people.get(5), names.get(5)))
  };

  public func change() : async () {
    old.add(1, { name = "Ana"; age = 30 });
    old.add(2, { name = "Bo"; age = 41 });
    let t : NativeTable.NativeTable<Nat, New> = old.evolve();
    changed := ?t;
    Prim.debugPrint(debug_show (t.size(), t.sum("age"), t.fieldTypes()))
  };

  // The added array fields hold null in the rows from before, and arrays once written
  public func changedRows() : async () {
    switch changed {
      case ?t {
        ignore t.update(1, { codes = ?["a", "b"] });
        t.add(3, { name = "Cy"; age = 7; codes = null; history = ?[{ amount = 5 }, { amount = 6 }] });
        let v : NativeTable.NativeTable<Nat, { codes : ?[Text]; history : ?[{ amount : Nat }] }> = t.view();
        for e in t.entries() { Prim.debugPrint(debug_show e) };
        Prim.debugPrint(debug_show (v.get(1), v.get(2), v.get(3)))
      }
      case null {}
    }
  };

  public func otherElement() : async () {
    switch changed {
      case ?t {
        let u : NativeTable.NativeTable<Nat, { name : Text; age : Nat; codes : ?[Nat] }> = t.evolve();
        ignore u
      }
      case null {}
    }
  };

  public func size() : async () {
    switch changed {
      case ?t { Prim.debugPrint(debug_show (t.size(), t.sum("age"))) }
      case null {}
    }
  };

  public func indexOnArray() : async () {
    ignore NativeTable.empty<Nat, Person>().createIndex(["codes"]).build()
  };

  public func filterOnArray() : async () {
    ignore people.countWhere("tags", #eq(#null_))
  };

  public func sumOfArray() : async () {
    ignore people.sum("scores")
  };

  public func minOfArray() : async () {
    ignore people.min("scores")
  };

  public func cellsOfArray() : async () {
    ignore people.cells(false, ["codes"], null).next()
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress build "DIDL\x00\x00"
//CALL ingress kindsRoundTrip "DIDL\x00\x00"
//CALL ingress put "DIDL\x00\x00"
//CALL ingress update "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress change "DIDL\x00\x00"
//CALL ingress changedRows "DIDL\x00\x00"
//CALL ingress otherElement "DIDL\x00\x00"
//CALL ingress size "DIDL\x00\x00"
//CALL ingress indexOnArray "DIDL\x00\x00"
//CALL ingress filterOnArray "DIDL\x00\x00"
//CALL ingress sumOfArray "DIDL\x00\x00"
//CALL ingress minOfArray "DIDL\x00\x00"
//CALL ingress cellsOfArray "DIDL\x00\x00"
