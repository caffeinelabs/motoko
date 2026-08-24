//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-a-b-at-m1.most

// Baseline applied m1 (m2 pending) but has no c: nothing pending produces
// it → M0267 at the mid-chain resume point.
actor {
    let b : Nat;
    var c : Nat;
};
