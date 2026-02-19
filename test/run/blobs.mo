//MOC-FLAG -A=M0194
import { debugPrint } = "mo:⛔";

debugPrint (debug_show ("" : Blob));
debugPrint (debug_show ("\00\01\02" : Blob));
debugPrint (debug_show ("\FF" : Blob));
debugPrint (debug_show ("\u{FF}" : Blob));
debugPrint (debug_show ("☃" : Blob));

assert (("":Blob) == ("":Blob));
assert (("\00":Blob) > ("":Blob));
assert (("\00":Blob) >= ("":Blob));
assert (("\00":Blob) < ("\01":Blob));
assert (("\00":Blob) <= ("\01":Blob));

do {
let i1 = ("\00\01☃":Blob).values();
switch(i1.next()) {
  case (?b) { assert (b == (0:Nat8)); };
  case null { assert false; };
};
switch(i1.next()) {
  case (?b) { assert (b == (1:Nat8)); };
  case null { assert false; };
};
switch(i1.next()) {
  case (?b) { assert (b == (0xe2:Nat8)); };
  case null { assert false; };
};
switch(i1.next()) {
  case (?b) { assert (b == (0x98:Nat8)); };
  case null { assert false; };
};
switch(i1.next()) {
  case (?b) { assert (b == (0x83:Nat8)); };
  case null { assert false; };
};
switch(i1.next()) {
  case (?b) { assert false; };
  case null {};
};
};


do {
  let i2 : Blob = "Hi\00\01☃";
  for (i in i2.keys()) debugPrint (debug_show i2.get(i));
};

do {
  type Blub = Blob;
  let i3 : Blub = "жарко☃";
  assert i3.size() == 13;
  debugPrint(debug_show ("жарко☃" : Blub)[1]);
};
