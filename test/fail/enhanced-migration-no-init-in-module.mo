//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test1

// Test that uninitialized variables are rejected in modules
// even with --enhanced-migration flag

module {
    public var x : Nat;
    public let y : Nat;
};
