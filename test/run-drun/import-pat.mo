// test pattern matching imports
import P "mo:⛔";

import A = "import-pat/A";

import {
  // type T; // TBC
  start;
  inc = next
} = "import-pat/A";

import {
  // type T; // TBC
  start = begin;
  inc
} = "import-pat/A";

actor {

  type U = A.T;
  P.debugPrint(debug_show(A.inc(A.start)));
  P.debugPrint(debug_show(next(start)));
  P.debugPrint(debug_show(inc(begin)));

}
