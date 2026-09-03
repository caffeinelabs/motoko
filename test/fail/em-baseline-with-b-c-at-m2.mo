//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-b-c-at-m2.most

// Baseline applied the whole chain: nothing pending, so neither a nor any
// other migration input is demanded — only the actor's own fields b and c,
// both in the baseline. The check is silent.
actor {
    let b : Nat;
    var c : Nat;
};
