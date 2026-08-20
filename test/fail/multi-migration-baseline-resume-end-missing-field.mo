//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-b-at-m2.most

// Baseline applied the whole chain but has no c: nothing pending produces
// it → M0267 at the resume point.
actor {
    let b : Nat;
    var c : Nat;
};
