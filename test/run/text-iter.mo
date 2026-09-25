import Prim "mo:⛔"

let s = "hello " # "world!";
assert(s.size() == 12);

Prim.debugPrint "via `debugPrint`:";
Prim.debugPrint s;
Prim.debugPrint "";

Prim.debugPrint "via iteration and `debugPrintChar`: #1";
for a in s.chars() {
  Prim.debugPrintChar a;
};
Prim.debugPrint "";

Prim.debugPrint "via iteration and `debugPrintChar`: #2";
var x = 0;
for a in s.chars() {
  x += 1;
  Prim.debugPrintNat x;
  Prim.debugPrint ":";
  Prim.debugPrintChar '\'';
  Prim.debugPrintChar a;
  Prim.debugPrintChar '\'';
  Prim.debugPrint " ";
};
Prim.debugPrint "";

let russian = "Приветствую," # " мир!\n";
assert(russian.size() == 18);

Prim.debugPrint "via iteration and `debugPrintChar` (Unicode): #3";
x := 0;
for a in russian.chars() {
  x += 1;
  Prim.debugPrintNat x;
  Prim.debugPrint ":";
  Prim.debugPrintChar '\'';
  Prim.debugPrintChar a;
  Prim.debugPrintChar '\'';
  Prim.debugPrint " ";
};
Prim.debugPrint "";
assert(x == 18);

let emojis = "🙈🎸😋";
assert(emojis.size() == 3);

Prim.debugPrint "via iteration and `debugPrintChar` (Unicode): #4";
x := 0;
for a in emojis.chars() {
  x += 1;
  Prim.debugPrintNat x;
  Prim.debugPrint ":";
  Prim.debugPrintChar '\'';
  Prim.debugPrintChar a;
  Prim.debugPrintChar '\'';
  Prim.debugPrint " ";
};
Prim.debugPrint "";
assert(x == 3);

Prim.debugPrint russian;
Prim.debugPrint "";
switch russian.chars().next() {
  case ?c { Prim.debugPrintChar c; Prim.debugPrint "" }
  case _ {}
};

switch emojis.chars().next() {
  case ?c { assert (c == '\u{1f648}'); Prim.debugPrintChar c; Prim.debugPrint "" }
  case _ {}
};
