import Prim "mo:⛔";
do {
  Prim.debugPrint("Zero byte: >\00<");
};
do {
  Prim.debugPrint("FF byte: >\ff<");
};
do {
  Prim.debugPrint("uFF char: >\u{ff}<");
};
