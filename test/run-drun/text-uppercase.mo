import Prim "mo:⛔";

func check(t : Text, uc : Text) {
  Prim.debugPrint(t);
  Prim.debugPrint(Prim.textUppercase(t));
  Prim.debugPrint(uc);
  assert Prim.textUppercase(t) == uc;
};

check(
  "abcdcefghijabcdcefghijabcdcefghijabcdcefghijabcdcefghij 1234 x",
  "ABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJABCDCEFGHIJ 1234 X"
);

// these examples stolen from Rust doc
check("σ", "Σ");

// but at the end of a word, it's ς, not σ:
check("ὀδυσσεύς", "ὈΔΥΣΣΕΎΣ");

check("农历新年", "农历新年");
