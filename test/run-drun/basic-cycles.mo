// test cycle overflow detection and larger cycle transfers

import Prim "mo:⛔";

actor a {

  let balance : () -> Nat = Prim.cyclesBalance;
  let available : () -> Nat = Prim.cyclesAvailable;
  let accept : <system>Nat -> Nat = Prim.cyclesAccept;
  let add : <system>Nat -> () = Prim.cyclesAdd;
  let burn : <system>Nat -> Nat = Prim.cyclesBurn;

  let refunded : () -> Nat = Prim.cyclesRefunded;


  public func provisional_top_up_actor(a : actor {}, amount : Nat) : async () {
    let amount_ = amount;
    let ic00 = actor "aaaaa-aa" : actor {
      provisional_top_up_canister :
        { canister_id: Principal; amount : Nat } -> async ();
    };
    await ic00.provisional_top_up_canister({
      canister_id = Prim.principalOfActor(a);
      amount = amount_})
  };


  public func test(amount : Nat) : async () {
     Prim.debugPrint(debug_show({available=available(); amount = amount}));
     assert (available() == amount);
  };

  // test overflow of 128 bit cycle amounts
  public func overflow() : async () {

     // detect immediate overflow of add
     try (await async {
       add<system>(0x1_00000000_00000000_00000000_00000000);
       assert false;
     })
     catch (e) {Prim.debugPrint(Prim.errorMessage(e))};

     // detect incremental overflow of add
     try (await async {
       add<system>(0xFFFFFFFF_FFFFFFFF_FFFFFFFFF_FFFFFFF);
       Prim.debugPrint("ok");
       add<system>(0x1);
       assert false;
     })
     catch (e) { Prim.debugPrint(Prim.errorMessage(e)) };

     // detect accept overflow
     try (await async {
       let _ = accept<system>(0x1_00000000_00000000_00000000_00000000);
       assert false;
     })
     catch (e) { Prim.debugPrint(Prim.errorMessage(e)) };

  };

  var tests = [
     0x0,
     0xFFFFFFFF,
     0xFFFFFFFF_FFFFFFFF,
     0x1_00000000_00000000,
     0xFFFFFFFF_FFFFFFFF_FFFFFFFF,
     0xFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF,
  ];

  public func iter() : async () {

     for (amount in tests.vals()) {
       Prim.debugPrint(debug_show {balance = balance()});
       if (balance() < amount) {
         await provisional_top_up_actor(a, amount - balance());
       };
       Prim.debugPrint(debug_show {topped_up_balance = balance()});
       if (amount > balance()) {
         Prim.debugPrint("can't top up more");
         return; // give up on test
       };
       add<system>(amount);
       Prim.debugPrint(debug_show({added = amount}));
       try {
         await test(amount);
       } catch e {
         Prim.debugPrint(Prim.errorMessage(e));
       }
     }
  };

  public func go() : async (){
    await overflow();
    await iter();
    assert 1000 == burn<system> 1000
  }
};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low

