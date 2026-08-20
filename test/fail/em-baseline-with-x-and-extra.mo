//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-x-and-extra.most
//MOC-FLAG -A=M0194

// x explained by baseline (silent); `extra` in baseline but not in new pre → M0169
actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X};
};
