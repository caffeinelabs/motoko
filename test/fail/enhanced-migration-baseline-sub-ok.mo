//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-sub
//MOC-FLAG --stable-baseline enhanced-migration/baselines/n-nat.most
//MOC-FLAG -A=M0194

// baseline Nat is a stable subtype of required Int → carry
actor {
  var a : Nat;
  var n : Int;
};
