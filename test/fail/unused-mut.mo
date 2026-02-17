// var never assigned (should warn M0244)
do {
  var x = 0;
  ignore x;
};

// var that is assigned (should not warn)
do {
  var y = 0;
  y := 1;
  ignore y;
};

// var that is unused (should get M0194, not M0244)
do {
  var z = 0;
};

// var with underscore prefix (should not warn)
do {
  var _w = 0;
  ignore _w;
};

// var assigned with += (should not warn)
do {
  var a = 0;
  a += 1;
  ignore a;
};

// var assigned with -= (should not warn)
do {
  var b : Int = 5;
  b -= 1;
  ignore b;
};

// let binding (should not warn M0244)
do {
  let v = 0;
  ignore v;
};
