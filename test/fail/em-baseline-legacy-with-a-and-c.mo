//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/legacy-with-a-and-c.most

// Legacy (non-EM) baseline: the whole chain replays on upgrade, demanding a
// (m1's input) and c (untouched by any migration). Both are in the baseline,
// so the check is silent.
actor {
    let b : Nat;
    var c : Nat;
};
