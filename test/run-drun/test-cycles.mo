//MOC-FLAG -A=M0194

import Prim = "mo:⛔";
import Cycles = "cycles/cycles";
import WalletLib = "cycles/wallet";

actor client {

 func print(t:Text) { Prim.debugPrint("client: " # t); };

 public func go() : async () {
  if (Cycles.balance() == 0)
    await Cycles.provisional_top_up_actor(client, 3_000_000_000_000);

//  print("balance: " # debug_show(Cycles.balance()) ); // to volatile to show

  print("available: " # debug_show(Cycles.available()));

  print("accept(0): " # debug_show(Cycles.accept<system>(0)));

  Cycles.add<system>(2_000_000_000_000);
  let wallet : WalletLib.Wallet = await WalletLib.Wallet();
  await wallet.show();
  print ("setting cycles");
  await Cycles.provisional_top_up_actor(wallet, 1_000_000_000);
  await wallet.show();

  // debit from the wallet, crediting this actor via callback
  let amount = 1000_000;
  print("# debit");
//  print("balance: " # debug_show(Cycles.balance()));
  let b = Cycles.balance();
  await wallet.debit(amount, credit);
//  print("balance: " # debug_show(Cycles.balance()));
  let b1 = Cycles.balance();
  assert (b <= b1 and b1 <= b + amount);

  print("# credit-1");
  // transfer half the amount back to the wallet
//  print(debug_show(await wallet.balance()));
  Cycles.add<system>(amount/4);
  await wallet.credit();
  print("refunded: " # debug_show(Cycles.refunded()));
//  print(debug_show(await wallet.balance()));


  print("# credit-2");
  // transfer half the amount back to the wallet
//  print(debug_show(await wallet.balance()));
  Cycles.add<system>(amount/4);
  await wallet.credit();
  print("refunded: " # debug_show(Cycles.refunded()));
//  print(debug_show(await wallet.balance()));


  print("# refund");
  // transfer half the amount back to the wallet
//  print(debug_show(await wallet.balance()));
  Cycles.add<system>(amount/2);
  await wallet.refund(amount/4);
  print("refunded: " # debug_show(Cycles.refunded()));
//  print(debug_show(await wallet.balance()));


  // issue a bunch of refund requests, await them in reverse and check the refunds are as expected.
  func testRefunds(n : Nat) : async () {
     if (n == 0) return;
     Cycles.add<system>(n);
     print("refund(" # debug_show(n) # ")");
     let a = wallet.refund(n);
     await testRefunds( n - 1);
     await a;
     print("refunded: " # debug_show(Cycles.refunded()));
     assert (Cycles.refunded() == n);
  };

  await testRefunds(5);

  // try to accept cycles that aren't available
  // this should trap
  print(debug_show(Cycles.accept<system>(1)));

 };

 // callback for accepting cycles from wallet.
 public func credit() : async () {
//   print("credit: balance " # debug_show(Cycles.balance()));
   let b = Cycles.balance();
   let a = Cycles.available();
   ignore Cycles.accept<system>(a);
//   print("credit:balance " # debug_show(Cycles.balance()));
//     assert (Cycles.balance() == b + a);
 };


};

client.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP drun-run
