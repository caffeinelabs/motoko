//MOC-FLAG --actor-idl issue-5824
import { type T } = "ic:aaaaa-aa";

actor {
  public func go(_ : T) : async () {
  };
}

//SKIP run
//SKIP run-ir
//SKIP run-low
//SKIP comp
