//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-empty
//MOC-FLAG -A=M0194

// The migrations directory exists but contains no .mo files: the chain is
// literally empty and must be rejected (M0251), since the actor's stable
// fields could never be initialized.

actor {
    var name : Text;
    var balance : Nat;
};
