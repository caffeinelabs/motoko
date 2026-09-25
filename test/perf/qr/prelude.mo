/**
[#mod-Prelude]
= `Prelude` -- General utilities
*/

import Prim "mo:⛔";

module {
  /*

Stdlib prelude
===============

 This prelude file proposes standard library features that _may_
belong in the _language_ (compiler-internal) prelude sometime, after
some further experience and discussion.  Until then, they live here.

*/

  /*

  `printLn`
  ---------

  Print text followed by a newline.

*/
  public func printLn(x : Text) {
    Prim.debugPrint(x # "\n");
  };

  /*

 `nyi`: Not yet implemented
 -----------------------------

 Mark incomplete code with the `nyi` and `xxx` functions.

 Each have calls are well-typed in all typing contexts, which
trap in all execution contexts.

*/

  public func nyi() : None { assert false; loop {} };

  public func xxx() : None { assert false; loop {} };

  /*

 `unreachable`
 --------------------

 Mark unreachable code with the `unreachable` function.

 Calls are well-typed in all typing contexts, and they
 trap in all execution contexts.

*/
  public func unreachable() : None { assert false; loop {} };
}
