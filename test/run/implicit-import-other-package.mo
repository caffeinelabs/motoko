//MOC-FLAG --package core ../core-stub/src
//MOC-FLAG --package base ../base-stub/src
//MOC-FLAG --implicit-package core
// With --implicit-package core, Map's implicit compare must resolve to core/Text only.
// base/Text is loaded transitively via base/Extra but never imported directly,
// and would otherwise compete with core/Text for the same implicit.

import Map "mo:core/Map";
import Extra "mo:base/Extra"; // transitively loads base/Text (not imported directly)

func main() {
  Extra.touch();
  let map = Map.empty<Text, Nat>();
  map.add("abc", 3); // implicit compare must resolve to core/Text only, unambiguously
};

main();
