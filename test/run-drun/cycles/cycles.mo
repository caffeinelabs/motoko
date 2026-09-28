import Prim "mo:⛔";

module {

  public let balance : () -> Nat = Prim.cyclesBalance;

  public let available : () -> Nat = Prim.cyclesAvailable;

  public let accept : <system>Nat -> Nat = Prim.cyclesAccept;

  public let add : <system>Nat -> () = Prim.cyclesAdd;

  public let refunded : () -> Nat = Prim.cyclesRefunded;

  public func provisional_top_up_actor(a : actor {}, amount : Nat) : async () {
    let amount_ = amount;
    let ic00 = actor "aaaaa-aa" : actor {
      provisional_top_up_canister :
        { canister_id: Principal; amount : Nat } -> async ();
    };
    await ic00.provisional_top_up_canister({
      canister_id = Prim.principalOfActor(a);
      amount = amount_
    })
  }
}
