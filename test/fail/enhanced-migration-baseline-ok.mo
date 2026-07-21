//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-x.most
//MOC-FLAG -A=M0194

actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X}; // explained by baseline
};
