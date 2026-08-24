//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-history
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-bad-dropped-lossy-at-m2.most
//MOC-FLAG -A=M0194

// Three ways the migration directory can disagree with the deployed history
// (see with-bad-dropped-lossy-at-m2.most): `m0` is backdated (sorts before the head but is not
// recorded), `m1` was edited after deploy (produces Int, the history records
// Nat), and the deployed head `m2` was deleted, which is not a prefix trim.
// The actor itself matches the deployed state exactly.
actor {
    let bad : Int;
    let dropped : Nat;
    let lossy : { a : Nat; b : Nat };
};
