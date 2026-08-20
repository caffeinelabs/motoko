//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-y-at-m1.most
//MOC-FLAG -A=M0194

// Same project as multi-migration-baseline-drop-pending.mo, one deploy
// later: the baseline is now this code's own signature — chain ends in "m1"
// and x is gone from its post actor. The upgrade skips m1 and x is never
// demanded — yet the initial-required check replays the chain against the
// baseline post and reports M0267 for x.
actor {
    var y : Nat;
};
