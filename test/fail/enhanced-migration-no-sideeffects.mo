//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test1
//MOC-FLAG -A=M0194

actor {
    let b : Bool;

    func myFunc() : Nat { 5 };
    transient let x : Nat = myFunc();

};
