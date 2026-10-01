import Nested "mixins/Nested";
actor {
  include Nested();
  public func test() : async () {
    await increment(); // transitive include
    await decrement();
  };
};
