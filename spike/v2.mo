persistent actor {
  let greeting : Text;
  let total : Nat;

  public func show() : async Text {
    greeting # "/" # debug_show total
  };
}
