import Prim "mo:⛔";

func check(t : Text, lc : Text) {
  Prim.debugPrint(t);
  Prim.debugPrint(Prim.textLowercase(t));
  Prim.debugPrint(lc);
  assert Prim.textLowercase(t) == lc;
};

check(
  "ABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJ 1234 X",
  "abcdcefghijabcdcefghijabcdcefghijabcdcefghijabcdcefghij 1234 x"
);

// these examples stolen from Rust doc
check("Σ", "σ");

// but at the end of a word, it's ς, not σ:
check("ὈΔΥΣΣΕΎΣ", "ὀδυσσεύς");

check("农历新年", "农历新年");
