import Prim "mo:⛔";

// The compiler supplies a native-table codec only for the key and row types it accepts
func codec<K, R>(c : (implicit : Prim.NtCodec<(K, R)>)) : Prim.NtCodec<(K, R)> = c;

// An array field
ignore codec<Nat, { name : Text; tags : [Text] }>();

// A nested record
ignore codec<Nat, { name : Text; address : { city : Text } }>();

// A mutable field
ignore codec<Nat, { name : Text; var visits : Nat }>();

// An optional key
ignore codec<?Nat, { name : Text }>();

// A generic row type
func table<R>() : Prim.NtCodec<(Nat, R)> = codec<Nat, R>()
