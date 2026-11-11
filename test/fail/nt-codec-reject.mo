import Prim "mo:⛔";

// The compiler supplies a native-table codec only for the key and row types it accepts
func codec<K, R>(c : (implicit : Prim.NtCodec<(K, R)>)) : Prim.NtCodec<(K, R)> = c;

// A mutable array field
ignore codec<Nat, { name : Text; tags : [var Text] }>();

// An array of arrays
ignore codec<Nat, { name : Text; grid : [[Nat]] }>();

// An array of variants
ignore codec<Nat, { name : Text; states : [{ #on; #off }] }>();

// An array of options of records
ignore codec<Nat, { name : Text; history : [?{ amount : Nat }] }>();

// An array of records with an array field
ignore codec<Nat, { name : Text; history : [{ amount : Nat; notes : [Text] }] }>();

// An array of records with no field
ignore codec<Nat, { name : Text; marks : [{}] }>();

// A nested record
ignore codec<Nat, { name : Text; address : { city : Text } }>();

// A mutable field
ignore codec<Nat, { name : Text; var visits : Nat }>();

// An optional key
ignore codec<?Nat, { name : Text }>();

// A key that is a tuple with an optional part
ignore codec<(Text, ?Nat), { name : Text }>();

// A key that is a tuple of 8 parts
ignore codec<(Nat, Nat, Nat, Nat, Nat, Nat, Nat, Nat), { name : Text }>();

// A generic row type
func table<R>() : Prim.NtCodec<(Nat, R)> = codec<Nat, R>()
