//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration enhanced-migration/enh-mig-test3
//MOC-FLAG -A=M0194

actor {

  let x : Int; // accept
  var y : Int; // accept
  let untyped : (); // accept

  do {
    let x : Int; // reject
    var y : Int; // reject
  };
  func f () {
    let x : Int; // reject
    var y : Int; // reject
    switch ()  {
      case _ {
        let x : Int; // reject
        var y : Int; // reject
      }
    }
  };

  do {
    let O = object {
      let x : Int; // reject
      var y : Int; // reject
    }
  };

  do {
     let M = module {
       let x : Int; // reject
       var y : Int; // reject
     };
  };

  do {
    let _ = do ? {
      let x : Int; // reject
      var y : Int; // reject
    };
  };

  func a () : async () {
    let x : Int; // reject
    var y : Int; // reject
    await async {
      let x : Int; // reject
      var y : Int; // reject
    };
    try {
      let x : Int; // reject
      var y : Int; // reject
    }
    catch e {
      let x : Int; // reject
      var y : Int; // reject
    };
  };
}
