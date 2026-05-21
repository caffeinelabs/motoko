persistent actor Counter_v3 {
  var state : Int = 0; // implicitly `stable`

  public func increment() : async () {
    state += 1;
  };

  public func decrement() : async () {
    state -= 1;
  };

  public query func read() : async Int {
    return state;
  };
};
