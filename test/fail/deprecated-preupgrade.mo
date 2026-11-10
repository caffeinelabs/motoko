//MOC-FLAG -A=M0194,M0198

// system func preupgrade/postupgrade are deprecated (M0270); heartbeat and
// other system funcs must NOT warn. A module-level `func preupgrade` (not an
// actor system field) must also NOT warn.

module NonActor {
  // plain functions; not actor system fields, so no M0270
  func preupgrade() {};
  func postupgrade() {};
};

actor {
  system func preupgrade() {};
  system func postupgrade() {};
  system func heartbeat() : async () {};
  public func greet() : async Text { "hi" }
}
