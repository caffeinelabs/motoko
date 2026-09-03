//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/legacy-with-x-only.most

// Legacy baseline providing neither demand: `a` is the chain's own input
// (m1 consumes it — no new migration file can fix that, so no hint), `c` is
// the actor's (a new migration can produce it, so the hint applies), and the
// deployed `x` is used by nothing.
actor {
    let b : Nat;
    var c : Nat;
};
