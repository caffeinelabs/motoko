import Prim "mo:⛔";
actor a {
  public func go() : async () {
    var cnt : Nat = 0;

    func f(i:Nat) : async Nat {
        Prim.debugPrint ("cnt: " # debug_show cnt # " i: " # debug_show i);
        cnt += 1;
        cnt;
    };

    Prim.debugPrint "a";

    let a = async await f(0);

    Prim.debugPrint "b";

    let b = async { await f(1); };

    Prim.debugPrint "c";

    let c = async {
        let _ = await f(2);
        await f(3);
    };

    Prim.debugPrint "d";

    let d  = (async { return await f(4); }) : async Int; 

    Prim.debugPrint "e";

    let e = async {
        var i = 5;
        Prim.debugPrint "e-while";
        while (i < 8) {
            let _ = await f(i);
            i += 1;
        };
        Prim.debugPrint "e-exit";
    };

    Prim.debugPrint "g";

    let g = async {
        var i = 10;
        Prim.debugPrint "g-label";
        while (true) {
            if (i < 13) {
                Prim.debugPrint ".";
                let _ = await f(i);
                i += 1;
                continue; 
            } else {};
            break;
        };
        Prim.debugPrint "g-exit";
    };

    Prim.debugPrint "holy";

    func p():async (Text,Text) { ("fst","snd"); };
    let h = async {
       let (a,b) = await p();
       Prim.debugPrint a;
       Prim.debugPrint b;
    };

    ignore(await a);
    ignore(await b);
    ignore(await c);
    ignore(await d);
    await e;
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
