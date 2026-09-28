import Prim "mo:⛔";
/* test generic self-tail-recursion */

let bound:Int = 100000;

do {
  func Loop(n:Int){
    if n >= bound {
      Prim.debugPrint "done 1";
      return;
    };
    Loop(n + 1);
  };
  Loop(0);
};

do {
  func Loop(n:Int){
    if n >= bound {
      Prim.debugPrint "done 2";
      return;
    };
    return Loop(n + 1);
    assert(false);
  };

  Loop(0);
};

do {
  func Loop<T>(n:Int){
    if n >= bound {
      Prim.debugPrint "done 3";
      return;
    };
    Loop<T>(n + 1);
  };

  Loop<Int>(0);
};

do {
  func Loop<T, U>(n:Int){
    if n >= bound {
      Prim.debugPrint "done 4";
      return;
    };
    Loop<T, U>(n + 1);
  };

  Loop<Int, Bool>(0);
};
