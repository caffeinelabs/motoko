//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-trimmed
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-bad-dropped-lossy-at-m2.most
//MOC-FLAG -A=M0194

// The deployed history is m1, m2 (see with-bad-dropped-lossy-at-m2.most); the local directory
// keeps only m2 — a legal prefix trim. Against the deployed state the actor
// retypes `bad`, drops `dropped`, narrows `lossy`, and declares `consumed`,
// which the deployed state does not have.
actor {
    let bad : Nat;
    let consumed : Nat;
    let lossy : { a : Nat };
};
