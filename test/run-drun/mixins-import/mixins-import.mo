import ImportMixin "ImportMixin";
import Text "Text1";

actor {
  include ImportMixin("MyPrefix ", " MySuffix");

  let t1 = Text.text1();
  let _x : MixinTy = 10;
  assert msg == "MyPrefix Hello from Mixin MySuffix";
  assert t1 == "Hello";
  assert t2 == "World";

  let _prefix : Nat = 10;
  public func actorFunc() : async Text {
    assert (await mixinFunc()) == msg;
    "Hello from the actor"
  };

};
