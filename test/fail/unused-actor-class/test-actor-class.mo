actor class TestActorClass(actorClassParameter1 : Nat) {
  var actorClassVariable1 = 0;
  var actorClassVariable2 = "TEST";

  func unusedActorClassMethod(parameter1 : Nat, parameter2 : Int) {
    actorClassVariable1 := 1;
  };

  public func publicActorClassMethod() : async() {};
};
