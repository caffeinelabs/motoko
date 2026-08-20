//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG --stable-baseline enhanced-migration/baselines/legacy-with-x-and-y.most
//MOC-FLAG -A=M0194

// The project started with the legacy `(with migration = ...)` syntax and is
// converted to enhanced migration with a single-entry chain: m1 drops x.
// The baseline is the legacy deployment (still has x), so m1 is pending and
// x is explained by the previous version → M0254 warnings only.
actor {
    var y : Nat;
};
