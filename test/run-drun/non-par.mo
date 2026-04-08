import { call_raw; debugPrint; principalOfActor; replyDeadline; errorMessage; errorCode } = "mo:⛔";
import Cycles = "cycles/cycles";
// This is a copy of `par.mo`, but with legacy `Cycles.add`. It should output equivalent results.

actor A {

    func foo(next : () -> async ()) : async () {
        debugPrint ("foo: " # debug_show(Cycles.available()));
        Cycles.add<system> 3000; await next()
    };

    func bar(next : () -> async ()) : async () = async {
        Cycles.add<system> 4000; await next()
    };







    public func oneshot() : () {
        debugPrint ("oneshot deadline set: " # debug_show (0 != replyDeadline()));
        debugPrint ("oneshot cycles: " # debug_show(Cycles.available()));
    };

    public func rawable() : async () {
        debugPrint ("rawable: " # debug_show(Cycles.available()));
    };

    public func test() : async () {
        debugPrint "test()";
        let message = "Hi!";

        func closA(i : Nat) : async Nat {
            assert Cycles.available() == 101;
            i + message.size()
        };

        func closB() : async Nat = async {
            assert Cycles.available() == 102;
            message.size()
        };

        Cycles.add<system> 101; assert 42 == (await closA(39));
        Cycles.add<system> 102; assert 3 == (await closB());

        Cycles.add<system> 1000; let c : async () =

          foo(func() : async () = async { assert message == "Hi!" and Cycles.available() == 3000 });
        await c;
        Cycles.add<system> 5000; await
        bar(func() : async () = async { assert message == "Hi!" and Cycles.available() == 4000 });
    };

    public func test2() : async () {
        debugPrint "test2()";
        Cycles.add<system> 1042; await async { assert Cycles.available() == 1042 };
    };

    public func test3() : async () {
        debugPrint "test3()";
        oneshot();
        Cycles.add<system> 3456; oneshot();
        (with timeout = 5) oneshot();
        Cycles.add<system> 4567; A.oneshot();
    };

    public func test4() : async () {
        debugPrint "test4()";
        ignore await call_raw(principalOfActor A, "rawable", "DIDL\00\00");
        Cycles.add<system> 3456; ignore await call_raw(principalOfActor A, "rawable", "DIDL\00\00");
    };

    public func test5() : async () {
        let t : async () =
          (with timeout = 3; moot = "M") async {
            debugPrint "test5()";
          };
        await t
    };

    public func ext() : async () {
        debugPrint ("ext: " # debug_show(Cycles.available()));
    };

    public func test6() : async () {
        Cycles.add<system> 6543; await A.ext()
    };

    public func test7() : async () {
        debugPrint "test7()";
        try
          await (with timeout = 1) async {
              // busy loop
              loop ignore await (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand();
              ()
          }
        catch e {
            debugPrint("CAUGHT: " # debug_show errorCode e # " " # debug_show errorMessage e)
        }
    }
}

// testing
//SKIP run
//SKIP run-ir
//SKIP run-low

//CALL ingress test "DIDL\x00\x00"
//CALL ingress test2 "DIDL\x00\x00"
//CALL ingress test3 "DIDL\x00\x00"
//CALL ingress test4 "DIDL\x00\x00"
//CALL ingress test5 "DIDL\x00\x00"
//CALL ingress test6 "DIDL\x00\x00"
//CALL ingress test7 "DIDL\x00\x00"
//SKIP-SANITY-CHECKS
