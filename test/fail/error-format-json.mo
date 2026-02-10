//MOC-FLAG --error-format json
//MOC-FLAG -W=M0236
//MOC-FLAG --package core $MOTOKO_CORE

import Array "mo:core/Array";
import Map "mo:core/Map";
import Nat "mo:core/Nat";
import Option "mo:core/Option";
import VarArray "mo:core/VarArray";

do {
  let true = true;
};

let _ : Nat = "abc";

let ar = [1];
let _ = Array.filter<Nat>(ar, func x = x > 0);
// Expected spans with suggested replacements:
// {"file":"error-format-json.mo","line_start":18,"column_start":9,"line_end":18,"column_end":14,"suggested_replacement":"ar"}
// {"file":"error-format-json.mo","line_start":18,"column_start":27,"line_end":18,"column_end":31,"suggested_replacement":""}
let _ = ar.filter<Nat>(func x = x > 0); // expected no replacements

module M0223 {
  public func inferred<T>(x : T) : T = x;

  public func main() {
    let n1 = inferred<Nat>(1);
    ignore n1;
  };
};

let peopleMap = Map.empty<Nat, Text>();

ignore Map.size(peopleMap);

Map.add(
  peopleMap,
  Nat.compare,
  1,
  "John",
);

Map.add(
  peopleMap,
  1,
  "John",
);

ignore Map.size(
  Map.empty<Nat, Text>().map(func(key, value) { value })
);

ignore peopleMap.get(Nat.compare, 1); // single-line
ignore peopleMap.get(
  Nat.compare, // multi-line
  1,
);

do {
  Map.add(
    Map.empty<Nat, Text>(),
    Nat.compare,
    1,
    "John",
  );
};
do {
  Map.add(
    Map.empty<Nat, Text>().map(func(key, value) { value }),
    Nat.compare,
    1,
    "John",
  );
};

do {
  let photo = { sharedWith = [0] };
  let _ = Array.find<Nat>(photo.sharedWith, func(p) { p == 0 });
};
do {
  let photo = { sharedWith = [var 0] };
  let _ = VarArray.map<Nat, Int>(photo.sharedWith, func(x) { x + 1 });
};
do {
  let photo = { sharedWith = [0] };
  let _ = Array.find<Nat>(photo.sharedWith, func(p) { let o = Array.find<Nat>(photo.sharedWith, func(p) { p == 0 }); let p2 = Option.unwrap<Nat>(o); p == p2 });
};
