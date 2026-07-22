//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-x-bad-n.most
//MOC-FLAG -A=M0194

// x explained by baseline → M0254; n has incompatible baseline type → M0267
actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X};
    var n : Nat;
};
