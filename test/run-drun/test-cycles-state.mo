//MOC-FLAG -A=M0194
import Prim = "mo:⛔";
import Cycles = "cycles/cycles";
import WalletLib = "cycles/wallet";

// Test state behind cycles and refunds works as expected, with
// cycles initially zero, additive, reset on context switches
// refunds initially zero, reset on context switches, set on await.

actor a {

 let print = Prim.debugPrint;

 public func go() : async () {
  if (Cycles.balance() == 0)
    await Cycles.provisional_top_up_actor(a, 3_000_000_000_000);

  Cycles.add<system>(2_000_000_000_000);
  let wallet : WalletLib.Wallet = await WalletLib.Wallet();
  await wallet.show();
  print ("setting cycles");
  await Cycles.provisional_top_up_actor(wallet, 2_000_000_000_000);
  await wallet.show();

  // debit from the wallet, crediting this actor via callback
  print ("debit");
//  print("balance " # debug_show(Cycles.balance()));
  let b = Cycles.balance();
  await wallet.debit(1_000_000_000_000, credit);

//  print(debug_show(Cycles.balance()));

  do { // check cycles available
    Cycles.add<system>(1000_000);
    let cs = await wallet.available();
    assert (cs == 1000_000);
    assert (Cycles.refunded() == 1000_000);
  };
  do { // check cycles reset to zero on send
    let cs = await wallet.available();
    assert (cs == 0);
    assert (Cycles.refunded() == 0);
  };

  do { // check cycles additive to zero on send
    Cycles.add<system>(1000_000);
    Cycles.add<system>(2000_000);
    let cs = await wallet.available();
    assert (cs == 3000_000);
    assert (Cycles.refunded() == 3000_000);
  };

  // check cycles reset on context switch
  Cycles.add<system>(1000_000);
  await async {
    assert(Cycles.available() == 1000_000);
    // check cycles received
    Cycles.add<system>(5000);
    let cs = await wallet.available();
    assert (cs == 5000);
    assert (Cycles.refunded() == 5000);

    // add some unconsumed cycles
    Cycles.add<system>(200);
  };
  // check refund from await async ...
  assert (Cycles.refunded() == 1000_000);
  // check unconsumed cycles, declared before await, cleared on context switch
  let cs = await wallet.available();
  assert (cs == 0);

 };

 // callback for accepting cycles from wallet.
 public func credit() : async () {
   let b = Cycles.balance();
   let a = Cycles.available();
   let acc = Cycles.accept<system>(a);
   // print("balance before " # debug_show(b));
   // print("available " # debug_show(a));
   // print("accepted " # debug_show(acc));
   // print("balalance after " # debug_show(Cycles.balance()));
   assert (Cycles.balance() == b + acc);
 };


};

a.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low

