//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-history
//MOC-FLAG -A=M0194
//MOC-FLAG --stable-baseline enhanced-migration/baselines/history-mismatch.most

// The directory drifted from the recorded history in all three diagnosed ways:
// m0 is backdated (sorts before the deployed head m3 but never ran), m1 was
// edited after it ran, and m2 was deleted although the newer m3 is kept.
// Each reports M0268. m4 is pending and free to differ; the chain walk and
// the baseline boundary themselves are consistent and stay silent.
actor {
    let a : Nat;
};
