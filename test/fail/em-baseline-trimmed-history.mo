//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/trimmed-history.most

// The oldest deployed migration m0 is trimmed away — a legal trim: the kept
// m1 and m2 still match their recorded types, so the history check is silent
// and so is the rest of the baseline check.
actor {
    let b : Nat;
    var c : Nat;
};
