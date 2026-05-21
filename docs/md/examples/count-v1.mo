import Debug "mo:core/Debug";

persistent actor Counter_v1 {
  var state : Nat = 0; // implicitly `stable`

  public func increment() : async () {
    state += 1;
    Debug.print(debug_show (state));
  };
};
