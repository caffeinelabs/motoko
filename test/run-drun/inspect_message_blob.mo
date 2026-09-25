// NB: inspect cannot be tested in drun.
//     (requires dfx or playground for HTTP interface)
actor {

  var c = 0;

  public func inc() : async () { c += 1 };
  public func set(n : Nat) : async () { c := n };
  public query func read() : async Nat { c };
  public func reset() : () { c := 0 }; // oneway

  system func inspect({ arg : Blob }) : Bool  {
    return arg.size() <= 512;
  }
};
