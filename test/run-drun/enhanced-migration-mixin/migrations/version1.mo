module {

  public func migration({}) : { mixinNat : Nat; actorInt : Int; actorText : Text } = {
   actorInt = 42;
   actorText = "hello";
   mixinNat = 42;
  };

}
