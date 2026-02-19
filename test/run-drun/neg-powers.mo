//MOC-FLAG -A=M0194
import Prim "mo:⛔";
// We have these tests in run-drun because we want to check that certain
// traps are happening, and a good way to test this is if a message gets
// aborted.

actor a {
  public func go() : async (){
    try {await async {
        ignore (2 ** 2 :Int);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 ** 0 :Int);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 ** -1 :Int);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 ** -2 :Int);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 ** 2 :Int8);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 ** 0 :Int8);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 ** -1 :Int8);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 ** -2 :Int8);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 ** 2 :Int16);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 ** 0 :Int16);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 ** -1 :Int16);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 ** -2 :Int16);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 ** 2 :Int32);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 ** 0 :Int32);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 ** -1 :Int32);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 ** -2 :Int32);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 ** 2 :Int64);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 ** 0 :Int64);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 ** -1 :Int64);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 ** -2 :Int64);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 **% 2 :Int8);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 **% 0 :Int8);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 **% -1 :Int8);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 **% -2 :Int8);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 **% 2 :Int16);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 **% 0 :Int16);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 **% -1 :Int16);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 **% -2 :Int16);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 **% 2 :Int32);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 **% 0 :Int32);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 **% -1 :Int32);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 **% -2 :Int32);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

    try {await async {
        ignore (2 **% 2 :Int64);
        Prim.debugPrint("ok 1");
    }} catch e {
        Prim.debugPrint("not ok 1");
    };
    try {await async {
        ignore (2 **% 0 :Int64);
        Prim.debugPrint("ok 2");
    }} catch e {
        Prim.debugPrint("not ok 2");
    };
    try {await async {
        ignore (2 **% -1 :Int64);
        Prim.debugPrint("not ok 3");
    }} catch e {
        Prim.debugPrint("ok 3");
    };
    try {await async {
        ignore (2 **% -2 :Int64);
        Prim.debugPrint("not ok 4");
    }} catch e {
        Prim.debugPrint("ok 4");
    };

  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"

// not useful, we don't do rollback properly in the interpreter
//SKIP run
//SKIP run-ir
//SKIP run-low
