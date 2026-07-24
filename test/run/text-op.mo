import { textCompare } = "mo:⛔"

assert (    ("" <= "a"));
assert (    ("" <  "a"));
assert (not ("" == "a"));
assert (not ("" >= "a"));
assert (not ("" >  "a"));

assert (    ("a" <= "a"));
assert (not ("a" <  "a"));
assert (    ("a" == "a"));
assert (    ("a" >= "a"));
assert (not ("a" >  "a"));

assert (    ("a" <= "aa"));
assert (    ("a" <  "aa"));
assert (not ("a" == "aa"));
assert (not ("a" >= "aa"));
assert (not ("a" >  "aa"));

assert (    ("a" <= "aa"));
assert (    ("a" <  "aa"));
assert (not ("a" == "aa"));
assert (not ("a" >= "aa"));
assert (not ("a" >  "aa"));

assert (    ("a" <= "ä"));
assert (    ("a" <  "ä"));
assert (not ("a" == "ä"));
assert (not ("a" >= "ä"));
assert (not ("a" >  "ä"));

// Equality short-circuits on differing byte lengths; ordering must not.
assert (    ("DE" == "DE"));
assert (not ("DE" == "D"));
assert (not ("DE" == "DEU"));
assert (    ("DE" != "DEU"));
assert (    ("ab" <  "abc"));
assert (not ("z"  <  "aaa"));

assert textCompare("", "a") == -1;
assert textCompare("b", "a") == 1;
assert textCompare("a", "") == 1;
assert textCompare("a", "b") == -1;
assert textCompare("", "") == 0;
assert textCompare("a", "a") == 0;
