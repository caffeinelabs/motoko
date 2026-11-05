import Principal "mo:core/Principal";
import IC "mo:core/InternetComputer";
/// Relay: forwards one measured call to the target canister, so the target's
/// cycles balance changes by the execution of that call alone (no ingress fee
/// lands on the target; the caller pays the transmission fees).
persistent actor {
  type M = { instr : Nat64; pmut : Nat64; pgc : Nat64; hb : Nat; ha : Nat };
  public func op(target : Principal, kind : Nat64, k : Nat64) : async M {
    let t : actor { op : (Nat64, Nat64) -> async M } = actor (Principal.toText(target));
    await t.op(kind, k);
  };
  public func calib(target : Principal, n : Nat64, pages : Nat64) : async M {
    let t : actor { calib : (Nat64, Nat64) -> async M } = actor (Principal.toText(target));
    await t.calib(n, pages);
  };
  public func batch(target : Principal, method : Text, a : Nat64, b : Nat64) : async M {
    switch method {
      case "wBuild" { let t : actor { wBuild : (Nat64, Nat64) -> async M } = actor (Principal.toText(target)); await t.wBuild(a, b) };
      case "wReindex" { let t : actor { wReindex : Nat64 -> async M } = actor (Principal.toText(target)); await t.wReindex(b) };
      case "wDelete" { let t : actor { wDelete : Nat64 -> async M } = actor (Principal.toText(target)); await t.wDelete(b) };
      case "wBuildR" { let t : actor { wBuildR : (Nat64, Nat64) -> async M } = actor (Principal.toText(target)); await t.wBuildR(a, b) };
      case "ordQuery" { let t : actor { ordQuery : () -> async M } = actor (Principal.toText(target)); await t.ordQuery() };
      case "wBuildT" { let t : actor { wBuildT : (Nat64, Nat64) -> async M } = actor (Principal.toText(target)); await t.wBuildT(a, b) };
      case _ { let t : actor { wBackfill : Nat64 -> async M } = actor (Principal.toText(target)); await t.wBackfill(a) };
    };
  };
  public func raw(target : Principal, method : Text, arg : Blob) : async Blob {
    await IC.call(target, method, arg);
  };
};
