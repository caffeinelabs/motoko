//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-sub
//MOC-FLAG --stable-baseline enhanced-migration/baselines/n-int.most
//MOC-FLAG -A=M0194

// baseline Int is not a stable subtype of required Nat → M0267
actor {
  var a : Nat;
  var n : Nat;
};
