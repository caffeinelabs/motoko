//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-history
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-bad-dropped-lossy-at-m2.most
//MOC-FLAG -A=M0194 -W=M0268

// Same divergent directory as the -history-mismatch test, with the history
// check demoted to a warning via -W=M0268: the escape hatch reports the
// disagreements without failing on them, while the chain walk's own errors
// still stand.
actor {
    let bad : Int;
    let dropped : Nat;
    let lossy : { a : Nat; b : Nat };
};
