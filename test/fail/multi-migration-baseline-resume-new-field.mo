//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-y-at-m1.most
//MOC-FLAG -A=M0194

// The baseline resumes after m1, so m1's inputs are not demanded — but z is
// new, produced by no migration, and absent from the baseline → M0267.
actor {
    var y : Nat;
    var z : Nat;
};
