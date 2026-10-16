// An ill-typed pattern in an actor field does not hide errors in later fields
persistent actor {
  let #okk(n) = (#ok(1) : { #ok : Nat; #err : Text }) else { loop {} };
  public func f() : async Nat { n + "checked" };
};
