//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
import Prim "mo:⛔";

type Result<T> = { #ok : T; #err : Text };
type Record = { value : Text; count : Nat64 };

assert Prim.fixtureDecode<Text>("hello") == #ok "hello";
assert Prim.fixtureDecode<Blob>("hello") == #ok ("hello" : Blob);
assert Prim.fixtureDecode<Nat>("hello") == #ok 5;
assert Prim.fixtureDecode<Record>("hello") ==
  #ok { value = "hello"; count = 5 };

// Aliases preserve the intrinsic identity and its explicit specialization.
let decode = Prim.fixtureDecode;
let methods = { decode };
let text : Result<Text> = decode<Text>("alias");
let record : Result<Record> = methods.decode<Record>("record");
assert text == #ok "alias";
assert record == #ok { value = "record"; count = 6 };
assert (if (true) decode else methods.decode)<Nat>("branch") == #ok 6;

// Both forward aliases and result subtyping retain the specialized layout.
let forward : Result<Later> = decode<Later>("forward");
type Later = Record;
assert forward == #ok { value = "forward"; count = 7 };
let projected : Result<{ value : Text }> = decode<Record>("projection");
assert projected == #ok { value = "projection" };

// An ordinary wrapper can expose a chosen specialization as a function.
func parse(text : Text) : Result<Record> = decode<Record>(text);
let parseRecord : Text -> Result<Record> = parse;
assert parseRecord("wrapper") == #ok { value = "wrapper"; count = 7 };
func apply<A, B>(f : A -> B, value : A) : B = f(value);
assert apply<Text, Result<Record>>(parseRecord, "generic") ==
  #ok { value = "generic"; count = 7 };

// A wider expected result does not erase the exact generated result layout.
let wide : Any = decode<Record>("wide");
ignore wide;

var effects = "";
let result = (do { effects #= "callee;"; decode })<Text>(
  do { effects #= "argument;"; "value" }
);
assert effects == "callee;argument;";
assert result == #ok "value";
