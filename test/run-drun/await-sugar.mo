// test scoping sugar - most tests disabled until we support explicit
// scope parameterisation/instantiation

actor A {

  public func f0 () : async () {};

  public func test0 () : async () { await f0 (); };

  /*
public func f1 () : async<$> () {}; // this is now illegal since f1 not (implicitly) parametric

public func test1 () : async () {
 // f1 not parametric, can call f1(), but can't await f1();
 ignore f1();
};
*/

  /*
public func f2<$>() : async<$> () {};

public func test2(): async () {
 await f2();
 await f2<$>();
};

public func f2b<$>() : async () {};

public func test2b() : async () {
 await f2b();
 await f2b<$>();
};
*/

  func f3<A<:Int>() : async () {};

  public func test3 () : async () {
    await f3<Int>(); // scope passed as implicit first argument
    /* await f3<$,Int>(); // scope passed as explicit first argument */
  };

  /*
public func f4<$,B<:Int>() : async<$> () {}; // explict scope parameter 1

public func test4() : async () {
 await f4<Int>();   // scope passed as implicit first argument
 await f4<$,Int>(); // scope passed as exlicit first argument
};


public func f5<A<:Int,$>() : async<$> () {}; // explict scope parameter 2

public func test5() : async () {
 await f5<Int>();   // scope passed as implicit second argument
// await f5<Int,$>();
}
;
*/

  /*
public func f6<A<:Int,$>() : async () {}; // explict scope parameter 2, implicit index

public func test6() : async () {
 await f6<Int>();   // scope passed as implicit second argument
 await f6<Int,$>();
}
;
*/

  /*
public func f7<X,A<:Int>(n:Int) : async<X>() {
   if (n == 0) ()
   else
   switch (n % 3) {
     case 0 (await f7<A>(n-1));
     case 1 (await f7<X,A>(n-1));
     case 2 (await f7<$,A>(n-1));
     case _ ();
   };
};

public func test7 () : async () {
 await f7<Int>(1);   // scope passed as implicit  argument
/* await f7<$,Int>(1); */
};

*/

  public func test() : async () {
    await test0();
    //  await test2();
    //  await test2b();
    await test3();
    //  await test4();
    //  await test5();
    //  await test6();
    //  await test7();
  };

};

A.test(); //OR-CALL ingress test 0x4449444C0000
//SKIP comp
