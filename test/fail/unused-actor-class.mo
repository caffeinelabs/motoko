import TestActor "unused-actor-class/test-actor-class";

actor {
  public func test(): async() {
    ignore await TestActor.TestActorClass(1);
  };
};
