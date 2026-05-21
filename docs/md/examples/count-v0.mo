import Debug "mo:core/Debug";

persistent actor Counter_v0 {
  transient var state : Nat = 0;

  public func increment() : async () {
    state += 1;
    Debug.print(debug_show (state));
  };
};
