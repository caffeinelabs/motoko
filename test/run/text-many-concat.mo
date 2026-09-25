import Prim "mo:⛔";

// right-leaning tree
do {
  var output : Text = "";
  var i = 1000000;
  while i > 0 {
    output := "WHOOHOOO" # output;
    i -= 1;
  };
  Prim.debugPrint("Ignore Diff: " # output);
};

// left-leaning tree
do {
  var output : Text = "";
  var i = 1000000;
  while i > 0 {
    output := "WHOOHOOO" # output;
    i -= 1;
  };
  Prim.debugPrint("Ignore Diff: " # output);
};

// complete tree (cannot go too dep, grows exponential)
do {
  var output : Text = "WHOOHOOO";
  var i = 20;
  while i > 0 {
    output := output # output;
    i -= 1;
  };
  Prim.debugPrint("Ignore Diff: " # output);
};

//SKIP run
//SKIP run-ir
//SKIP run-low
