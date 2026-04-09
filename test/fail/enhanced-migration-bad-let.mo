//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test3
//MOC-FLAG -A=M0194

// uninitialized lets must have <id> : <typ>
actor {

  let x : Int; // accept

  let (y,_) : (Int,Int); // reject

  let 1 : Int; // reject

  let untyped; // reject, no annotation

}
