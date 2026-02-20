//MOC-FLAG --ai-errors
//MOC-FLAG --package core ../core-stub/src
//MOC-FLAG --error-format human

import Map "mo:core/Map";

// The following import should be suggested.
// It should NOT be imported by the Map above.
// import Text "../src/Text";

let map = Map.empty<Text, Nat>();
map.add("abc", 3)

// let ar = [3, 1, 2];
// let ar2 = ar.sort()
