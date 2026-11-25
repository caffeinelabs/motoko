//MOC-FLAG --experimental-local-actors
// The exported classes of a container share one Candid interface: a name may be
// exported only once (M0280, then the ordinary M0051).
actor Container {
  public actor* class K() {
    public func ping() : async Nat { 1 };
  };
  public actor* class K() {
    public func pong() : async Nat { 2 };
  };
};
