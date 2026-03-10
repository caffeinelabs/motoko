actor a {
  let fooArr /* : [None] */  = [];
  ignore (fooArr : [Any]);
  public func run() : () {
    for (f in fooArr.values()) {
      await f();
    };
  };
};
