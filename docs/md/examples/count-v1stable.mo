import Debug "mo:core/Debug";

persistent actor Counter_v1 {
  stable var state : Nat = 0; // explicitly `stable`

  public func increment() : async () {
    state += 1;
    Debug.print(debug_show (state));
  };
};
