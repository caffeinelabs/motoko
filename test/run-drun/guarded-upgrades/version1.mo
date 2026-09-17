// default eop, not explicit, upgrade should fail
//MOC-FLAG --incremental-gc -unguarded-enhanced-orthogonal-persistence
actor {
   var value : Nat = 666;
};
