//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-x-bad-n.most
//MOC-FLAG -A=M0194

// Same chain as M0254; x carries, Int is not a stable subtype of Nat for n
actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X};
    var n : Nat;
};
