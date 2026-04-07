//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG -A=M0194

actor {
    let a : Float;
    let b : Bool;
    var c : Nat;
    let x : {#X}; // extra var, never involved in migrations, inherited from inital
};
