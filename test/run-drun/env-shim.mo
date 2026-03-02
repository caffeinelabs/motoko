import { Backend } = "env-shim/Backend";

actor {
  transient let backend = Backend<system>();

  public func test() : async () {
    await backend.increment();
    let _count = await backend.get();
    // Type-correct compilation is the goal - this won't actually run
  }
}

//SKIP run
//SKIP run-ir
//SKIP run-low
