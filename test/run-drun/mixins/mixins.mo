import MyMixin "MyMixin";

actor MixinActor {
  include MyMixin("MyPrefix ", " MySuffix");

  let _x : MixinTy = 10;
  assert msg == "MyPrefix Hello from Mixin MySuffix";

  let prefix : Nat = 10;
  public func actorFunc() : async Text {
    assert (await mixinFunc()) == msg;
    "Hello from the actor"
  };

};
