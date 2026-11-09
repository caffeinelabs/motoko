import Prim "mo:⛔";

// Only the compiler makes a native-table codec: a program reads its layout with
// `Prim.ntCodecLayout`, but cannot build one, read it as a record, or put it in a handle of other
// types
type Handle<K, R> = { id : Nat64; codec : Prim.NtCodec<(K, R)> };
type User = { name : Text; city : Text; amount : Nat };

func codec<K, R>(c : (implicit : Prim.NtCodec<(K, R)>)) : Prim.NtCodec<(K, R)> = c;

let users : Handle<Nat, User> = { id = 1; codec = codec<Nat, User>() };

// A record literal
let c1 : Prim.NtCodec<(Nat, Text)> = { layout = Prim.ntCodecLayout(users.codec); labels = "" : Blob };

// A generic function
func retype<K, R, K0, R0>(c : Prim.NtCodec<(K0, R0)>) : Prim.NtCodec<(K, R)> = c;

// The codec of another type in a handle
let h2 : Handle<Nat, Text> = { id = users.id; codec = users.codec };

// A narrower row type
let h3 : Handle<Nat, { name : Text; city : Text }> = users;

// Reading a part of a codec as a record field
let l : Blob = users.codec.layout
