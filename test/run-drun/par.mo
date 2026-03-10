//MOC-FLAG -A=M0194
import { call_raw; debugPrint; principalOfActor; replyDeadline; errorMessage; errorCode } = "mo:⛔";
import Cycles = "cycles/cycles";

actor A {

    func foo(next : () -> async ()) : async () {
        assert 0 != replyDeadline();
        debugPrint ("foo: " # debug_show(Cycles.available()));
        await (with cycles = 3000) next()
    };

    func bar(next : () -> async ()) : async () = async {
        await (with cycles = 4000) next()
    };

    func quux() : async () = (with) async {
        debugPrint ("quux: " # debug_show(Cycles.available()));
    };

    /*public func baz() : async () = (with) async {
        debugPrint ("baz: " # debug_show(Cycles.available()));
    };*/

    public func oneshot() : () {
        debugPrint ("oneshot deadline set: " # debug_show(0 != replyDeadline()));
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

        assert 42 == (await (with cycles = 101) closA(39));
        assert 3 == (await (with cycles = 102) closB());

        let c : async () =
          (with yeah = 8; timeout = 55; cycles = 1000)
          foo(func() : async () = async { assert message == "Hi!" and Cycles.available() == 3000 });
        await c;
        await (with cycles = 5000)
        bar(func() : async () = async { assert message == "Hi!" and Cycles.available() == 4000 });
    };

    public func test2() : async () {
        debugPrint "test2()";
        await (with cycles = 1042) async { assert Cycles.available() == 1042 };
    };

    public func test3() : async () {
        debugPrint "test3()";
        oneshot();
        (with cycles = 3456) oneshot();
        (with timeout = 5) oneshot();
        (with timeout = 5; cycles = 4567) A.oneshot();
    };

    public func test4() : async () {
        debugPrint "test4()";
        ignore await call_raw(principalOfActor A, "rawable", "DIDL\00\00");
        ignore await (with cycles = 3456) call_raw(principalOfActor A, "rawable", "DIDL\00\00");
    };

    public func test5() : async () {
        let t : async () =
          (with timeout = 3; moot = "M") async {
            debugPrint "test5()";
            assert 0 != replyDeadline();
          };
        await t
    };

    public func ext() : async () {
        assert 0 != replyDeadline();
        debugPrint ("ext: " # debug_show(Cycles.available()));
    };

    public func test6() : async () {
        await (with timeout = 3; cycles = 6543) A.ext()
    };

    public func test7() : async () {
        debugPrint "test7()";
        try
          await (with timeout = 1) async {
              assert 0 != replyDeadline();
              // busy loop
              loop ignore await (actor "aaaaa-aa" : actor { raw_rand : () -> async Blob }).raw_rand();
              ()
          }
        catch e {
            debugPrint("CAUGHT: " # debug_show errorCode e # " " # debug_show errorMessage e)
        }
    };

    public func test8() : async () {
        debugPrint "test8()";
        await (with cycles = 98765) quux();
        //await (with cycles = 87654) baz(); NOT YET!
        //await (with cycles = 76543) A.baz() NOT YET!
    };

    func localOuter() : async () { debugPrint ("localOuter: " # debug_show(Cycles.available())) };
    public func test9() : async () {
        debugPrint "test9()";
        var env = 42;
        func local() : async () { debugPrint ("local: " # debug_show(env + Cycles.available())) };

        // give a (dynamically) bogus base
        object base { /*public*/ func cycles() : Text = "bogus" };
        //let check : { cycles : Nat } = base;
        await (with cycles = 876) localOuter();
        await (base with) localOuter(); // bogus dynamic attr `cycles` gets ignored as a type-driven fresh record is passed
        await (with cycles = 987) local();
        await (base with) local(); // bogus dynamic attr `cycles` gets ignored as a type-driven fresh record is passed
    };

    public func test10() : async () {
        debugPrint "test10()";

        await (with cycles = 34567) (func(name : Text) : async () { debugPrint (name # ": " # debug_show(Cycles.available())) }) "anon";
    };

    func star() : async* () {
        debugPrint ("star: " # debug_show(Cycles.available()));
        oneshot();
    };

    public func test11() : async () {
        debugPrint "test11()";
        await* (with timeout = 1; cycles = do { debugPrint "evaled"; 987 }) star();
    };

    func outer() : async () {
        debugPrint ("outer cycles: " # debug_show(Cycles.available()));
    };

    public func test12() : async () {
        debugPrint "test12()";
        await (with cycles = 9876) outer(await async { debugPrint ("inner cycles: " # debug_show(Cycles.available())) });
        func p(c : Nat) : Nat { debugPrint (debug_show c); c };
        await (with cycles = p 8765) outer(await (with cycles = p 7654) async { debugPrint ("inner cycles: " # debug_show(Cycles.available())) });
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
//CALL ingress test8 "DIDL\x00\x00"
//CALL ingress test9 "DIDL\x00\x00"
//CALL ingress test10 "DIDL\x00\x00"
//CALL ingress test11 "DIDL\x00\x00"
//CALL ingress test12 "DIDL\x00\x00"
//SKIP-SANITY-CHECKS
