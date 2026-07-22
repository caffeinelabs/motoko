//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG --stable-baseline enhanced-migration/baselines/without-x.most
//MOC-FLAG -A=M0194

// Same actor as M0254; baseline lacks x and n
actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X};
    var n : Int;
};
