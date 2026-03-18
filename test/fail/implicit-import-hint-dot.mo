//MOC-FLAG --all-libs
//MOC-FLAG --package core ../core-stub/src

import Map "mo:core/Map";

// The following import should be suggested.
// It should NOT be imported transitively by the imports above
// import Text "../src/Text";

let map = Map.empty<Text, Text>();

func main1() {
  map.get()
};

func main2() {
  let ?x = map.get("abc") else return;
  x.compare("v12")
};
