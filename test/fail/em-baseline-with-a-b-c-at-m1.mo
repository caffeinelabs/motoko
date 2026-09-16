//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-a-b-c-at-m1.most

// Baseline applied m1, m2 pending: the upgrade demands a (m2's input), b
// (produced by the already-applied m1, so it must live in the deployed
// state), and c. All present in the baseline, so the check is silent.
actor {
    let b : Nat;
    var c : Nat;
};
