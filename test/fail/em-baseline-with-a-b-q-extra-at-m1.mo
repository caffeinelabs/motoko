//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test4
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-a-b-q-extra-at-m1.most

// Baseline applied m1 (m2 pending) and carries legacy fields: the deployed
// q would lose data against the actor's narrower type (M0216), and nothing
// demands extra → dropped on upgrade (M0169), both named against the resume
// point.
actor {
    let b : Nat;
    let q : { x : Nat };
};
