module {
  public func test() : async () {
    try {} catch exception {
      switch exception {
        case unused {
          assert false;
        }
      };
    };
  };
};
