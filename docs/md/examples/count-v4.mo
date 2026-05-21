import Float "mo:core/Float";

persistent actor Counter_v4 {
  var state : Float = 0.0; // implicitly `stable`

  public func increment() : async () {
    state += 0.5;
  };

  public func decrement() : async () {
    state -= 0.5;
  };

  public query func read() : async Float {
    return state;
  };
};
