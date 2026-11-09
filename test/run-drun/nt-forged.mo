import Prim "mo:⛔";
import NativeTable "mo:native-table/NativeTable";

// A handle that pairs a table's id with the codec of other key and row types traps on every
// write and read, and writes nothing; so does a handle whose id is not a table's
actor {
  type User = { name : Text; city : Text; amount : Nat };

  let users = NativeTable.empty<Nat, User>().build();

  // Tables with a row of a scalar type
  let nat32s = NativeTable.empty<Nat, Nat32>().build();
  let nat64s = NativeTable.empty<Nat, Nat64>().build();
  let texts = NativeTable.empty<Nat, Text>().build();

  // A handle on table t with key type K and row type R, holding the codec the compiler makes for them
  func retype<K, R, K0, R0>(t : NativeTable.NativeTable<K0, R0>, codec : (implicit : Prim.NtCodec<(K, R)>)) : NativeTable.NativeTable<K, R> {
    { id = t.id; codec; indexes = t.indexes }
  };

  // A row of a scalar type, a key of another type, a field of another type, a field fewer, and
  // fields of other names
  public func textRow() : async () { retype<Nat, Text, Nat, User>(users).add(1, "Ana") };
  public func textKey() : async () {
    retype<Text, User, Nat, User>(users).add("1", { name = "Ana"; city = "Basel"; amount = 40 })
  };
  public func intField() : async () {
    retype<Nat, { name : Text; city : Text; amount : Int }, Nat, User>(users).add(1, { name = "Ana"; city = "Basel"; amount = -1 })
  };
  public func missingField() : async () {
    retype<Nat, { name : Text; amount : Nat }, Nat, User>(users).add(1, { name = "Ana"; amount = 40 })
  };
  public query func otherLabels() : async () {
    Prim.debugPrint(debug_show retype<Nat, { title : Text; city : Text; amount : Nat }, Nat, User>(users).get(1))
  };

  // A row of another scalar type of the same size
  public func otherScalar(i : Nat) : async () {
    switch i {
      case 0 { retype<Nat, Char, Nat, Nat32>(nat32s).add(1, 'a') }
      case 1 { retype<Nat, Int64, Nat, Nat64>(nat64s).add(1, -1 : Int64) }
      case _ { retype<Nat, Blob, Nat, Text>(texts).add(1, "a" : Blob) }
    }
  };

  public func put() : async () {
    users.add(1, { name = "Ana"; city = "Basel"; amount = 40 })
  };

  public query func check() : async () {
    Prim.debugPrint(debug_show (users.get(1), users.size(), nat32s.size(), nat64s.size(), texts.size()))
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
//CALL ingress intField "DIDL\x00\x00"
//CALL ingress missingField "DIDL\x00\x00"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x00"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x01"
//CALL ingress otherScalar "DIDL\x00\x01\x7d\x02"
//CALL ingress check "DIDL\x00\x00"
//CALL ingress put "DIDL\x00\x00"
//CALL ingress otherLabels "DIDL\x00\x00"
//CALL ingress putForgery "DIDL\x00\x00"
//CALL ingress forgedAtBytes "DIDL\x00\x00"
//CALL ingress check "DIDL\x00\x00"
