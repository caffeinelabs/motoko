import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A handle that pairs a table's id with the codec of other key and row types traps on every
// write and read, and writes nothing
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().build();

  // Tables with a row of each scalar type, and of an option
  let bools = NativeTable.empty<Nat, Bool>().build();
  let nat8s = NativeTable.empty<Nat, Nat8>().build();
  let nat16s = NativeTable.empty<Nat, Nat16>().build();
  let nat32s = NativeTable.empty<Nat, Nat32>().build();
  let nat64s = NativeTable.empty<Nat, Nat64>().build();
  let int8s = NativeTable.empty<Nat, Int8>().build();
  let int16s = NativeTable.empty<Nat, Int16>().build();
  let int32s = NativeTable.empty<Nat, Int32>().build();
  let int64s = NativeTable.empty<Nat, Int64>().build();
  let ints = NativeTable.empty<Nat, Int>().build();
  let floats = NativeTable.empty<Nat, Float>().build();
  let chars = NativeTable.empty<Nat, Char>().build();
  let texts = NativeTable.empty<Nat, Text>().build();
  let blobs = NativeTable.empty<Nat, Blob>().build();
  let principals = NativeTable.empty<Nat, Principal>().build();
  let options = NativeTable.empty<Nat, { x : ?Nat8 }>().build();

  // A handle on table t with key type K and row type R, holding the codec the compiler makes for them
  func retype<K, R, K0, R0>(t : NativeTable.NativeTable<K0, R0>, codec : (implicit : Prim.NtCodec<(K, R)>)) : NativeTable.NativeTable<K, R> {
    { id = t.id; codec; indexes = t.indexes }
  };

  func forge<K, R>(codec : (implicit : Prim.NtCodec<(K, R)>)) : NativeTable.NativeTable<K, R> {
    { id = users.id; codec; indexes = users.indexes }
  };

  public func textRow() : async () {
    forge<Nat, Text>().add(1, "Ana")
  };

  public func textKey() : async () {
    forge<Text, User>().add("1", { name = "Ana"; city = "Basel"; amount = 40 })
  };

  public func natField() : async () {
    forge<Nat, { name : Nat; city : Text; amount : Nat }>().add(1, { name = 5; city = "Basel"; amount = 40 })
  };

  public func textField() : async () {
    forge<Nat, { name : Text; city : Text; amount : Text }>().add(1, { name = "Ana"; city = "Basel"; amount = "40" })
  };

  public func missingField() : async () {
    forge<Nat, { name : Text; amount : Nat }>().add(1, { name = "Ana"; amount = 40 })
  };

  public func mutableField() : async () {
    forge<Nat, { name : Text; city : Text }>().add(1, { name = "Ana"; city = "Basel"; var amount = 40 })
  };

  public func negativeField() : async () {
    forge<Nat, { name : Text; city : Text; amount : Int }>().add(1, { name = "Ana"; city = "Basel"; amount = -1 })
  };

  public func negativeBigField() : async () {
    forge<Nat, { name : Text; city : Text; amount : Int }>().add(1, { name = "Ana"; city = "Basel"; amount = -(2 ** 100) })
  };

  public query func textKeyGet() : async () {
    Prim.debugPrint(debug_show forge<Text, User>().get("1"))
  };

  public query func textKeyContains() : async () {
    Prim.debugPrint(debug_show forge<Text, User>().containsKey("1"))
  };

  public query func otherLabelsGet() : async () {
    Prim.debugPrint(debug_show forge<Nat, { title : Text; city : Text; amount : Nat }>().get(1))
  };

  public query func fewerLabelsGet() : async () {
    Prim.debugPrint(debug_show forge<Nat, { name : Text; amount : Nat }>().get(1))
  };

  public query func textRowGet() : async () {
    Prim.debugPrint(debug_show forge<Nat, Text>().get(1))
  };

  public query func get() : async () {
    Prim.debugPrint(debug_show users.get(1))
  };

  // A value of another scalar type
  public func otherScalar(i : Nat) : async () {
    switch i {
      case 0 { retype<Nat, Nat8, Nat, Bool>(bools).add(1, 255 : Nat8) }
      case 1 { retype<Nat, Int8, Nat, Nat8>(nat8s).add(1, -1 : Int8) }
      case 2 { retype<Nat, Nat8, Nat, Nat16>(nat16s).add(1, 1 : Nat8) }
      case 3 { retype<Nat, Char, Nat, Nat32>(nat32s).add(1, 'a') }
      case 4 { retype<Nat, Nat8, Nat, Int8>(int8s).add(1, 1 : Nat8) }
      case 5 { retype<Nat, Int8, Nat, Int16>(int16s).add(1, 1 : Int8) }
      case 6 { retype<Nat, Nat32, Nat, Int32>(int32s).add(1, 1 : Nat32) }
      case 7 { retype<Nat, Nat32, Nat, Char>(chars).add(1, 65 : Nat32) }
      case 8 { retype<Nat, Int64, Nat, Nat64>(nat64s).add(1, 1 : Int64) }
      case 9 { retype<Nat, Int64, Nat, Nat64>(nat64s).add(1, -9_223_372_036_854_775_808 : Int64) }
      case 10 { retype<Nat, Float, Nat, Int64>(int64s).add(1, 1.0) }
      case 11 { retype<Nat, Text, Nat, Int>(ints).add(1, "1") }
      case 12 { retype<Nat, Nat64, Nat, Float>(floats).add(1, 9_223_372_036_854_775_808 : Nat64) }
      case 13 { retype<Nat, Blob, Nat, Text>(texts).add(1, "a" : Blob) }
      case 14 { retype<Nat, Principal, Nat, Blob>(blobs).add(1, Prim.principalOfBlob("\04")) }
      case 15 { retype<Nat, Blob, Nat, Principal>(principals).add(1, "\04" : Blob) }
      case _ {}
    }
  };

  public query func checkScalars() : async () {
    Prim.debugPrint(debug_show [
      bools.size(),
      nat8s.size(),
      nat16s.size(),
      nat32s.size(),
      nat64s.size(),
      int8s.size(),
      int16s.size(),
      int32s.size(),
      int64s.size(),
      ints.size(),
      floats.size(),
      chars.size(),
      texts.size(),
      blobs.size(),
      principals.size(),
      options.size(),
    ])
  };

  public func put() : async () {
    users.add(1, { name = "Ana"; city = "Basel"; amount = 40 })
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show users.size())
  };

  // Long Blobs in rows, whose bytes repeat the two words a table's root block holds at byte 3,968:
  // the table magic and the codec hash of `users`. The Blobs start the repetition at each of its 16
  // bytes, so one of them puts the two words at that byte of every block its stored bytes fill,
  // wherever its bytes start.
  let longs = NativeTable.empty<Nat, Blob>().build();

  public func putForgery() : async () {
    let layout = Prim.blobToArray(Prim.ntCodecLayout(users.codec));
    let magic = Prim.blobToArray("MOTOKONT");
    let words = Prim.Array_tabulate<Nat8>(16, func i = if i < 8 { magic[i] } else { layout[i] });
    var k = 0;
    while k < 16 {
      longs.add(k, Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(65_536, func i = words[(i + k) % 16])));
      k += 1
    }
  };

  public func sizeAt(id : Nat64) : async Nat {
    { users with id }.size()
  };

  // Whether t contains p
  func contains(t : Text, p : Text) : Bool {
    let a = Prim.blobToArray(Prim.encodeUtf8(t));
    let b = Prim.blobToArray(Prim.encodeUtf8(p));
    var i = 0;
    while i + b.size() <= a.size() {
      var j = 0;
      while j < b.size() and a[i + j] == b[j] { j += 1 };
      if j == b.size() { return true };
      i += 1
    };
    false
  };

  // A handle at each of the 1,024 blocks after the root block of the table that holds the long
  // Blobs traps as not a table
  public func forgedAtBytes() : async () {
    var notTable = 0;
    var other = 0;
    var i : Nat64 = 1;
    while i <= 1_024 {
      try {
        ignore await sizeAt(longs.id + i * 4_096);
        other += 1
      } catch e {
        if contains(Prim.errorMessage(e), "native table: not a table of this key and row type") { notTable += 1 } else {
          if other == 0 { Prim.debugPrint(debug_show (i, Prim.errorMessage(e))) };
          other += 1
        }
      };
      i += 1
    };
    Prim.debugPrint(debug_show { notTable; other })
  }
}

// native tables need compiled code
//SKIP run
//SKIP run-ir
//SKIP run-low
//MOC-FLAG --package native-table ../../lib/native-table
//CALL ingress textRow "DIDL\x00\x00"
//CALL ingress textKey "DIDL\x00\x00"
//CALL ingress natField "DIDL\x00\x00"
//CALL ingress textField "DIDL\x00\x00"
//CALL ingress missingField "DIDL\x00\x00"
//CALL ingress mutableField "DIDL\x00\x00"
//CALL ingress negativeField "DIDL\x00\x00"
//CALL ingress negativeBigField "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress put "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress textKeyGet "DIDL\x00\x00"
//CALL ingress textKeyContains "DIDL\x00\x00"
//CALL ingress otherLabelsGet "DIDL\x00\x00"
//CALL ingress fewerLabelsGet "DIDL\x00\x00"
//CALL ingress textRowGet "DIDL\x00\x00"
//CALL ingress get "DIDL\x00\x00"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x00"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x01"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x02"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x03"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x04"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x05"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x06"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x07"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x08"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x09"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0a"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0b"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0c"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0d"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0e"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x0f"
//CALL ingress checkScalars "DIDL\x00\x00"
//CALL ingress putForgery "DIDL\x00\x00"
//CALL ingress forgedAtBytes "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
