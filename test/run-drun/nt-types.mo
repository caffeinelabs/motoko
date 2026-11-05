import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// Every scalar type a table accepts, and its option, round trip at their extremes: as keys, as
// rows and as record fields, with big Nat and Int, NaN, Text built by concatenation, non-ASCII
// text and null. The table names the type of every field of the record, and none for a row that is
// a scalar
actor {
  type Row = {
    bool : Bool;
    nat8 : Nat8;
    nat16 : Nat16;
    nat32 : Nat32;
    nat64 : Nat64;
    int8 : Int8;
    int16 : Int16;
    int32 : Int32;
    int64 : Int64;
    nat : Nat;
    int : Int;
    float : Float;
    char : Char;
    text : Text;
    blob : Blob;
    principal : Principal;
    optBool : ?Bool;
    optNat8 : ?Nat8;
    optNat16 : ?Nat16;
    optNat32 : ?Nat32;
    optNat64 : ?Nat64;
    optInt8 : ?Int8;
    optInt16 : ?Int16;
    optInt32 : ?Int32;
    optInt64 : ?Int64;
    optNat : ?Nat;
    optInt : ?Int;
    optFloat : ?Float;
    optChar : ?Char;
    optText : ?Text;
    optBlob : ?Blob;
    optPrincipal : ?Principal
  };

  let rows = NativeTable.empty<Nat, Row>().build();

  // Tables whose key and row have one type
  let bools = NativeTable.empty<Bool, Bool>().build();
  let nat8s = NativeTable.empty<Nat8, Nat8>().build();
  let nat16s = NativeTable.empty<Nat16, Nat16>().build();
  let nat32s = NativeTable.empty<Nat32, Nat32>().build();
  let nat64s = NativeTable.empty<Nat64, Nat64>().build();
  let int8s = NativeTable.empty<Int8, Int8>().build();
  let int16s = NativeTable.empty<Int16, Int16>().build();
  let int32s = NativeTable.empty<Int32, Int32>().build();
  let int64s = NativeTable.empty<Int64, Int64>().build();
  let nats = NativeTable.empty<Nat, Nat>().build();
  let ints = NativeTable.empty<Int, Int>().build();
  let floats = NativeTable.empty<Float, Float>().build();
  let chars = NativeTable.empty<Char, Char>().build();
  let texts = NativeTable.empty<Text, Text>().build();
  let blobs = NativeTable.empty<Blob, Blob>().build();
  let principals = NativeTable.empty<Principal, Principal>().build();

  // The limits of each type, and the limits of the forms values of the type take
  let boolValues : [Bool] = [false, true];
  let nat8Values : [Nat8] = [0, 1, 127, 128, 255];
  let nat16Values : [Nat16] = [0, 255, 256, 32_767, 65_535];
  let nat32Values : [Nat32] = [0, 65_535, 2_147_483_647, 2_147_483_648, 4_294_967_295];
  let nat64Values : [Nat64] = [
    0,
    1_152_921_504_606_846_975,
    1_152_921_504_606_846_976,
    9_223_372_036_854_775_808,
    18_446_744_073_709_551_615,
  ];
  let int8Values : [Int8] = [-128, -1, 0, 1, 127];
  let int16Values : [Int16] = [-32_768, -129, 0, 128, 32_767];
  let int32Values : [Int32] = [-2_147_483_648, -32_769, 0, 32_768, 2_147_483_647];
  let int64Values : [Int64] = [
    -9_223_372_036_854_775_808,
    -576_460_752_303_423_489,
    -576_460_752_303_423_488,
    -1,
    0,
    576_460_752_303_423_487,
    576_460_752_303_423_488,
    9_223_372_036_854_775_807,
  ];
  let natValues : [Nat] = [0, 2 ** 61 - 1, 2 ** 61, 2 ** 63, 2 ** 64 - 1, 2 ** 64, 2 ** 100];
  let intValues : [Int] = [
    -(2 ** 100),
    -(2 ** 64),
    -(2 ** 63) - 1,
    -(2 ** 63),
    -(2 ** 61) - 1,
    -(2 ** 61),
    -1,
    0,
    2 ** 61 - 1,
    2 ** 61,
    2 ** 63 - 1,
    2 ** 63,
    2 ** 100,
  ];
  let floatValues : [Float] = [
    -1.0 / 0.0,
    -1.7976931348623157e308,
    -1.5,
    0.0,
    5e-324,
    2.2250738585072014e-308,
    1.5,
    1.7976931348623157e308,
    1.0 / 0.0,
  ];
  let charValues : [Char] = ['\u{0}', 'A', 'é', '\u{D7FF}', '\u{E000}', '\u{FFFF}', '😀', '\u{10FFFF}'];
  let textValues : [Text] = ["", "a", "Zürich", "日本語", "😀", "Gen" # "è" # "ve", "\u{0}"];
  let blobValues : [Blob] = ["", "\00", "\FF", "\00\01\FF", "Gen\c3\a8ve"];
  let principalValues : [Principal] = [
    Prim.principalOfBlob(""),
    Prim.principalOfBlob("\04"),
    Prim.principalOfBlob("\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF\FF"),
  ];

  // Long values: a Nat, an Int, a Text built by concatenation and a Blob, each spread over more
  // than one page of table memory
  let huge = 2 ** 460_000;
  func long() : Text {
    var t = "Zürich ";
    var i = 0;
    while i < 14 { t := t # t; i += 1 };
    t
  };
  let longBlob = Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(100_000, func i = Prim.natToNat8(i % 256)));

  let lows : Row = {
    bool = false;
    nat8 = 0;
    nat16 = 0;
    nat32 = 0;
    nat64 = 0;
    int8 = -128;
    int16 = -32_768;
    int32 = -2_147_483_648;
    int64 = -9_223_372_036_854_775_808;
    nat = 0;
    int = -(2 ** 100);
    float = -1.0 / 0.0;
    char = '\u{0}';
    text = "";
    blob = "";
    principal = Prim.principalOfBlob("");
    optBool = null;
    optNat8 = null;
    optNat16 = null;
    optNat32 = null;
    optNat64 = null;
    optInt8 = null;
    optInt16 = null;
    optInt32 = null;
    optInt64 = null;
    optNat = null;
    optInt = null;
    optFloat = null;
    optChar = null;
    optText = null;
    optBlob = null;
    optPrincipal = null
  };

  let highs : Row = {
    bool = true;
    nat8 = 255;
    nat16 = 65_535;
    nat32 = 4_294_967_295;
    nat64 = 18_446_744_073_709_551_615;
    int8 = 127;
    int16 = 32_767;
    int32 = 2_147_483_647;
    int64 = 9_223_372_036_854_775_807;
    nat = 2 ** 100;
    int = 2 ** 100;
    float = 1.7976931348623157e308;
    char = '\u{10FFFF}';
    text = "日本語 😀";
    blob = "\FF\FF";
    principal = principalValues[2];
    optBool = ?true;
    optNat8 = ?255;
    optNat16 = ?65_535;
    optNat32 = ?4_294_967_295;
    optNat64 = ?18_446_744_073_709_551_615;
    optInt8 = ?(-128);
    optInt16 = ?(-32_768);
    optInt32 = ?(-2_147_483_648);
    optInt64 = ?(-9_223_372_036_854_775_808);
    optNat = ?(2 ** 100);
    optInt = ?(-(2 ** 100));
    optFloat = ?(-1.0 / 0.0);
    optChar = ?'😀';
    optText = ?("Zür" # "ich");
    optBlob = ?"\00";
    optPrincipal = ?Prim.principalOfBlob("\04")
  };

  // Zeros, the smallest tagged and boxed values, NaN, negative zero and long values
  let others : Row = {
    bool = false;
    nat8 = 1;
    nat16 = 256;
    nat32 = 2_147_483_648;
    nat64 = 1_152_921_504_606_846_976;
    int8 = -1;
    int16 = -129;
    int32 = -32_769;
    int64 = -576_460_752_303_423_489;
    nat = huge;
    int = -huge;
    float = 0.0 / 0.0;
    char = 'é';
    text = long();
    blob = longBlob;
    principal = Prim.principalOfBlob("\04");
    optBool = ?false;
    optNat8 = ?0;
    optNat16 = ?0;
    optNat32 = ?0;
    optNat64 = ?0;
    optInt8 = ?0;
    optInt16 = ?0;
    optInt32 = ?0;
    optInt64 = ?576_460_752_303_423_488;
    optNat = ?huge;
    optInt = ?(-huge);
    optFloat = ?(-0.0);
    optChar = ?'\u{0}';
    optText = ?long();
    optBlob = ?longBlob;
    optPrincipal = ?Prim.principalOfBlob("")
  };

  // Adds every value under itself
  func addAll<T>(t : NativeTable.NativeTable<T, T>, values : [T]) {
    for v in values.values() { t.add(v, v) }
  };

  // The rows the values have as keys
  func readBack<T>(t : NativeTable.NativeTable<T, T>, values : [T]) : [?T] {
    Prim.Array_tabulate<?T>(values.size(), func i = t.get(values[i]))
  };

  // Whether every value reads back equal to itself
  func same<T>(t : NativeTable.NativeTable<T, T>, values : [T], eq : (T, T) -> Bool) : Bool {
    for v in values.values() {
      switch t.get(v) {
        case ?w { if not eq(v, w) { return false } }
        case null { return false }
      }
    };
    true
  };

  public func put() : async () {
    addAll(bools, boolValues);
    addAll(nat8s, nat8Values);
    addAll(nat16s, nat16Values);
    addAll(nat32s, nat32Values);
    addAll(nat64s, nat64Values);
    addAll(int8s, int8Values);
    addAll(int16s, int16Values);
    addAll(int32s, int32Values);
    addAll(int64s, int64Values);
    addAll(nats, natValues);
    addAll(ints, intValues);
    addAll(floats, floatValues);
    addAll(chars, charValues);
    addAll(texts, textValues);
    addAll(blobs, blobValues);
    addAll(principals, principalValues);
    rows.add(0, lows);
    rows.add(1, highs);
    rows.add(2, others);
    Prim.debugPrint(debug_show (
      (bools.size(), nat8s.size(), nat16s.size(), nat32s.size(), nat64s.size()),
      (int8s.size(), int16s.size(), int32s.size(), int64s.size()),
      (nats.size(), ints.size(), floats.size(), chars.size()),
      (texts.size(), blobs.size(), principals.size(), rows.size()),
    ))
  };

  public query func get() : async () {
    Prim.debugPrint(debug_show readBack(bools, boolValues));
    Prim.debugPrint(debug_show readBack(nat8s, nat8Values));
    Prim.debugPrint(debug_show readBack(nat16s, nat16Values));
    Prim.debugPrint(debug_show readBack(nat32s, nat32Values));
    Prim.debugPrint(debug_show readBack(nat64s, nat64Values));
    Prim.debugPrint(debug_show readBack(int8s, int8Values));
    Prim.debugPrint(debug_show readBack(int16s, int16Values));
    Prim.debugPrint(debug_show readBack(int32s, int32Values));
    Prim.debugPrint(debug_show readBack(int64s, int64Values));
    Prim.debugPrint(debug_show readBack(nats, natValues));
    Prim.debugPrint(debug_show readBack(ints, intValues));
    Prim.debugPrint(debug_show readBack(floats, floatValues));
    Prim.debugPrint(debug_show readBack(chars, charValues));
    Prim.debugPrint(debug_show readBack(texts, textValues));
    Prim.debugPrint(debug_show readBack(blobs, blobValues));
    Prim.debugPrint(debug_show readBack(principals, principalValues));

    // Values read back are equal to the values written, and compute as they do
    Prim.debugPrint(debug_show [
      same<Bool>(bools, boolValues, func(a, b) = a == b),
      same<Nat8>(nat8s, nat8Values, func(a, b) = a == b),
      same<Nat16>(nat16s, nat16Values, func(a, b) = a == b),
      same<Nat32>(nat32s, nat32Values, func(a, b) = a == b),
      same<Nat64>(nat64s, nat64Values, func(a, b) = a == b),
      same<Int8>(int8s, int8Values, func(a, b) = a == b),
      same<Int16>(int16s, int16Values, func(a, b) = a == b),
      same<Int32>(int32s, int32Values, func(a, b) = a == b),
      same<Int64>(int64s, int64Values, func(a, b) = a == b),
      same<Nat>(nats, natValues, func(a, b) = a == b),
      same<Int>(ints, intValues, func(a, b) = a == b),
      same<Float>(floats, floatValues, func(a, b) = a == b),
      same<Char>(chars, charValues, func(a, b) = a == b),
      same<Text>(texts, textValues, func(a, b) = a == b),
      same<Blob>(blobs, blobValues, func(a, b) = a == b),
      same<Principal>(principals, principalValues, func(a, b) = a == b),
    ]);
    Prim.debugPrint(debug_show (
      nat64s.get(1_152_921_504_606_846_975 : Nat64) == ?(1_152_921_504_606_846_974 + 1 : Nat64),
      nat64s.get(18_446_744_073_709_551_615 : Nat64) == ?(18_446_744_073_709_551_614 + 1 : Nat64),
      int64s.get(-576_460_752_303_423_489 : Int64) == ?(-576_460_752_303_423_488 - 1 : Int64),
      int8s.get(-128 : Int8) == ?(-127 - 1 : Int8),
      chars.get('😀') == ?Prim.nat32ToChar(0x1F600),
      texts.get("Genève") == ?("Genè" # "ve"),
      texts.get("Gen" # "ève"),
      principals.get(Prim.principalOfBlob("\04")),
    ));

    // Keys that are not there
    Prim.debugPrint(debug_show (
      nat8s.get(2 : Nat8),
      int64s.get(1 : Int64),
      nats.get(2 ** 101),
      ints.get(-(2 ** 101)),
      floats.get(2.5),
      chars.get('B'),
      texts.get("Zurich"),
      blobs.get("\01" : Blob),
      principals.get(Prim.principalOfBlob("\05")),
    ));

    // Records with every type and its option
    Prim.debugPrint(debug_show rows.get(0));
    Prim.debugPrint(debug_show (rows.get(0) == ?lows));
    Prim.debugPrint(debug_show rows.get(1));
    Prim.debugPrint(debug_show (rows.get(1) == ?highs));
    switch rows.get(2) {
      case ?r {
        Prim.debugPrint(debug_show (r.float != r.float, Prim.floatCopySign(1.0, r.float)));
        switch r.optFloat {
          case ?f { Prim.debugPrint(debug_show (f == 0.0, Prim.floatCopySign(1.0, f))) }
          case null { Prim.debugPrint("missing") }
        };
        Prim.debugPrint(debug_show (r.nat == huge, r.int == -huge, r.nat + r.int, r.optNat == ?huge, r.optInt == ?(-huge)));
        Prim.debugPrint(debug_show (r.text == long(), r.text.size(), r.optText == ?long()));
        Prim.debugPrint(debug_show (r.blob == longBlob, r.blob.size(), r.optBlob == ?longBlob));
        Prim.debugPrint(debug_show { r with float = 0.0; optFloat = null; nat = 0; int = 0; text = ""; blob = ""; optNat = null; optInt = null; optText = null; optBlob = null })
      }
      case null { Prim.debugPrint("missing") }
    };

    // The fields of the record with every type and its option, in any order, and those of rows
    // that are scalars, which have none
    let types = rows.fieldTypes();
    Prim.debugPrint(debug_show types);
    Prim.debugPrint(debug_show sameFields(types, rowTypes));
    Prim.debugPrint(debug_show (bools.fieldTypes(), nats.fieldTypes(), floats.fieldTypes(), texts.fieldTypes(), principals.fieldTypes()))
  };

  func field(name : Text, scalar : NativeTable.Scalar, optional : Bool) : (Text, NativeTable.FieldType) {
    (name, { scalar; optional })
  };

  // The fields of `Row` and their types
  let rowTypes : [(Text, NativeTable.FieldType)] = [
    field("bool", #bool, false),
    field("nat8", #nat8, false),
    field("nat16", #nat16, false),
    field("nat32", #nat32, false),
    field("nat64", #nat64, false),
    field("int8", #int8, false),
    field("int16", #int16, false),
    field("int32", #int32, false),
    field("int64", #int64, false),
    field("nat", #nat, false),
    field("int", #int, false),
    field("float", #float, false),
    field("char", #char, false),
    field("text", #text, false),
    field("blob", #blob, false),
    field("principal", #principal, false),
    field("optBool", #bool, true),
    field("optNat8", #nat8, true),
    field("optNat16", #nat16, true),
    field("optNat32", #nat32, true),
    field("optNat64", #nat64, true),
    field("optInt8", #int8, true),
    field("optInt16", #int16, true),
    field("optInt32", #int32, true),
    field("optInt64", #int64, true),
    field("optNat", #nat, true),
    field("optInt", #int, true),
    field("optFloat", #float, true),
    field("optChar", #char, true),
    field("optText", #text, true),
    field("optBlob", #blob, true),
    field("optPrincipal", #principal, true),
  ];

  // Whether `a` and `b` hold the same fields, in any order
  func sameFields(a : [(Text, NativeTable.FieldType)], b : [(Text, NativeTable.FieldType)]) : Bool {
    if a.size() != b.size() { return false };
    for x in a.values() {
      var found = 0;
      for y in b.values() { if x == y { found += 1 } };
      if found != 1 { return false }
    };
    true
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress put "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
