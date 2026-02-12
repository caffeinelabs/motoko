//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration multi-migration/migrations

// Test that uninitialized variables are rejected in modules
// even with --enhanced-migration flag

module {
    public var x : Nat;
    public let y : Nat;
};
