//MOC-FLAG -W=M0223,M0236,M0237
//MOC-FLAG --error-format json
//MOC-FLAG --all-libs
//MOC-FLAG --package core ../core-stub/src

import Map "mo:core/Map";
import Nat "mo:core/Nat";

do {
  let true = true;
};

let _ : Nat = "abc";

let map = Map.empty<Text, Nat>();
let map2 = Map.empty<Nat, Nat>();

func m1() { map.add("abc", 3) };

Map.add(map2, Nat.compare, 1, 2);
