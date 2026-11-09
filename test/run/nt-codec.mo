import Prim "mo:⛔";

// The compiler fills an implicit native-table codec where the key and row types are known: its
// layout gives the key's type, then each field's type, label hash and name
func codec<K, R>(c : (implicit : Prim.NtCodec<(K, R)>)) : Prim.NtCodec<(K, R)> = c;

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

Prim.debugPrint(debug_show Prim.ntCodecLayout(codec<Text, Account>()))
