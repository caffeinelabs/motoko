import Prim "mo:⛔";

// The compiler fills an implicit native-table codec where the key and row types are known
func codec<K, R>(c : (implicit : Prim.NtCodec<(K, R)>)) : Prim.NtCodec<(K, R)> = c;

type User = { name : Text; city : Text; amount : Nat };
type Account = {
  active : Bool;
  score : ?Float;
  age : Nat8;
  delta : Int64;
  initial : ?Char;
  avatar : ?Blob;
  owner : Principal;
  balance : ?Int
};

// Motoko's label hash: h * 223 + byte over the name's UTF-8 bytes, keeping the low 31 bits
func labelHash(name : Text) : Nat {
  var h = 0;
  for b in Prim.encodeUtf8(name).values() {
    h := (h * 223 + Prim.nat8ToNat(b)) % 2_147_483_648
  };
  h
};

// The hash list of a record with these fields: the hashes in ascending order, 8 bytes each, little-endian
func hashList(names : [Text]) : Blob {
  let hs = Prim.Array_init<Nat>(names.size(), 0);
  for i in names.keys() {
    let h = labelHash(names[i]);
    var j = i;
    while j > 0 and hs[j - 1] > h {
      hs[j] := hs[j - 1];
      j -= 1
    };
    hs[j] := h
  };
  Prim.arrayToBlob(Prim.Array_tabulate<Nat8>(hs.size() * 8, func k = Prim.natToNat8(hs[k / 8] / 256 ** (k % 8) % 256)))
};

let user = codec<Nat, User>();
Prim.debugPrint(debug_show Prim.ntCodecLayout(user));
Prim.debugPrint(debug_show Prim.ntCodecLabels(user));
assert Prim.ntCodecLabels(user) == hashList(["name", "city", "amount"]);

let account = codec<Text, Account>();
Prim.debugPrint(debug_show Prim.ntCodecLayout(account));
Prim.debugPrint(debug_show Prim.ntCodecLabels(account));
assert Prim.ntCodecLabels(account) == hashList(["active", "score", "age", "delta", "initial", "avatar", "owner", "balance"])
