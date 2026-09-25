/* test actor import of eponymous types in separate candid files */
//MOC-FLAG --actor-idl issue-3103

import A "ic:lg264-qjkae";
import B "ic:aaaaa-aa";

actor a {
  public func go() : async () {
    let _ : Bool = await A.getBool();
    let _ : Int = await B.getInt();
  };
}

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp
