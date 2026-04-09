//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test2
//MOC-FLAG -A=M0194

actor {
    // a expected Text, but migrations make it Float.
    let a : Text;
    let b : Bool;
    var c : Nat;
};
