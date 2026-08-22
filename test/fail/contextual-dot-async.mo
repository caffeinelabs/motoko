module Async {
  public func asyncFunc(self : Text) : async Text {
    return self
  };

  public func otherAsync() : async Text {
    let x = "Hello";
    await x.asyncFunc() // fine
  };

  public func nonAsync() : Text {
    let x = "Hello";
    x.asyncFunc() // errors
  };
};
