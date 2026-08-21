//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-newstep
//MOC-FLAG --stable-baseline enhanced-migration/baselines/with-consumed-at-m1.most
//MOC-FLAG -A=M0194

// m2 is new since the deploy (see with-consumed-at-m1.most): it requires a field the
// deployed state lacks and misdeclares the type of one it has. The actor
// matches m2's output.
actor {
    let done : Nat;
};
