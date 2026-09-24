import Prim "mo:⛔";
// test flattening of awaitable, shared function arguments

let a = actor {
    public func m0():async() {};
    public func m1(x:Int):async Int {return x;};
    public func m2(x:Int,y:Bool):async(Int,Bool) {return (x,y);};
    public func m3(x:Int,y:Bool,z:Text):async(Int,Bool,Text) {return (x,y,z);};

    public func n0(u:()):async() {return u};
    public func n1(x:Int):async Int {return x;};
    public func n2(xy:(Int,Bool)):async(Int,Bool) {return xy;};
    public func n3(xyz:(Int,Bool,Text)):async(Int,Bool,Text) {return xyz;};

    // higher-order cases
    public func h0 (f0:shared () -> async (),u:()) : async ()
       { await f0 u;};
    public func h1 (f1:shared (Int) -> async Int,x:Int)  : async Int
       { await f1 x;};
    public func h2 (f2:shared (Int,Bool) -> async (Int,Bool), xy:(Int,Bool)) : async (Int,Bool)
       { await f2 xy; };
    public func h3 (f3:shared (Int,Bool,Text) -> async (Int,Bool,Text), xyz:(Int,Bool,Text)) : async (Int,Bool,Text)
       { await f3 xyz; };

    public func g0 (f0:shared (()) -> async (),u:()) : async ()
       { await f0 u;};
    public func g1 (f1:shared (Int) -> async Int,x:Int)  : async Int
       { await f1 x;};
    public func g2 (f2:shared ((Int,Bool)) -> async (Int,Bool), xy:(Int,Bool)) : async (Int,Bool)
       { await f2 xy; };
    public func g3 (f3:shared ((Int,Bool,Text)) -> async (Int,Bool,Text), xyz:(Int,Bool,Text)) : async (Int,Bool,Text)
       { await f3 xyz; };

  public func go() : async () {

    Prim.debugPrint "first-order";

    await a.m0();
    Prim.debugPrint "0";
    let 1 = await a.m1(1);
    Prim.debugPrint "1";
    let (2,true) = await a.m2(2,true);
    Prim.debugPrint "2";
    let (3,false,"text") = await a.m3(3,false,"text");
    Prim.debugPrint "3";

    await a.n0();
    Prim.debugPrint "4";
    let 1 = await a.n1(1);
    Prim.debugPrint "5";
    let (2,true) = await a.n2(2,true);
    Prim.debugPrint "6";
    let (3,false,"text") = await a.n3(3,false,"text");
    Prim.debugPrint "7";

    let u = ();
    let x:Int = 1;
    let xy:(Int,Bool) = (2,true);
    let xyz:(Int,Bool,Text) = (3,false,"text");

    await a.m0 u;
    Prim.debugPrint "8";
    let 1 = await a.m1 x;
    Prim.debugPrint "9";
    let (2,true) = await a.m2 xy;
    Prim.debugPrint "10";
    let (3,false,"text") = await a.m3 xyz;
    Prim.debugPrint "11";

    await a.n0 u;
    Prim.debugPrint "12";
    let 1 = await a.n1 x;
    Prim.debugPrint "13";
    let (2,true) = await a.n2 xy;
    Prim.debugPrint "14";
    let (3,false,"text") = await a.n3 xyz;
    Prim.debugPrint "15";

    Prim.debugPrint "higher-order";
    await a.h0(a.m0,());
    Prim.debugPrint "0";
    let 1 = await a.h1(a.m1,1);
    Prim.debugPrint "1";
    let (2,true) = await a.h2(a.m2,(2,true));
    Prim.debugPrint "2";
    let (3,false,"text") = await a.h3(a.m3,(3,false,"text"));
    Prim.debugPrint "3";

    await a.g0(a.n0,());
    Prim.debugPrint "4";
    let 1 = await a.g1(a.n1,1);
    Prim.debugPrint "5";
    let (2,true) = await a.g2(a.n2,(2,true));
    Prim.debugPrint "6";
    let (3,false,"text") = await a.g3(a.n3,(3,false,"text"));
    Prim.debugPrint "7";

    await a.h0(a.m0,u);
    Prim.debugPrint "8";
    let 1 = await a.h1(a.m1,x);
    Prim.debugPrint "9";
    let (2,true) = await a.h2(a.m2,xy);
    Prim.debugPrint "10";
    let (3,false,"text") = await a.h3(a.m3,xyz);
    Prim.debugPrint "11";

    await a.g0(a.n0,u);
    Prim.debugPrint "12";
    let 1 = await a.g1(a.n1,x);
    Prim.debugPrint "13";
    let (2,true) = await a.g2(a.n2,xy);
    Prim.debugPrint "14";
    let (3,false,"text") = await a.g3(a.n3,xyz);
    Prim.debugPrint "15";
  };
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
