//MOC-FLAG -A=M0194
import Prim "mo:⛔";

func f():() -> Int {
  do {
    func g() : Int = x; // reference x
    return (func() : Int{ g(); }); // early exit omits definition of x
    let x:Int = 666;
    func():Int{777;};
  };
};

Prim.debugPrint "1";
let h = f();

Prim.debugPrint "2";
let wrong = h();
