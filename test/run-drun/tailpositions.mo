import Prim "mo:⛔";
/* test tail-position calculation; error would stack overflow in drun*/

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
    if true
               { Loop(n + 1) }
    else
            { Loop(n + 1) };
  };
  Loop(0);
};

do {
  func Loop(n:Int){
    if n >= bound {
      Prim.debugPrint "done 3";
      return;
    };
    switch n % 2 {
      case 0 { Loop(n + 1) }
      case 1 { Loop(n + 1) }
      case _ { assert(false) }
    };
  };

  Loop(0);
};

do {
  func Loop(n:Int){
    if n >= bound {
      Prim.debugPrint "done 4";
      return;
    };
    do {
      let m = n;
      Loop(m +1);
    };
  };
  Loop(0);
};

do {
  func Loop(n:Int){
    if n >= bound {
      Prim.debugPrint "done 5";
      return;
    };
    let _ = (return Loop(n + 1)) + 1;
  };
  Loop(0);
};

do {
  func Loop(n:Int):Bool{
    if n >= bound {
      Prim.debugPrint "done 6";
      return true;
    };
    true and Loop(n + 1);
  };
  assert(Loop(0));
};

do {
  func Loop(n:Int):Bool {
    if n >= bound {
      Prim.debugPrint "done 7";
      return true;
    };
    false or Loop(n + 1);
  };
  assert(Loop(0));
};
