//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-fullytrimmed
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-bad-dropped-lossy-at-m2.most
//MOC-FLAG -A=M0194

// The deployed history m1, m2 (see with-bad-dropped-lossy-at-m2.most) is trimmed away entirely;
// only the new m3 remains. m3 consumes `bad` and `dropped` from the deployed
// state; the actor then drops the carried `lossy`.
actor {
    let bad : Nat;
};
