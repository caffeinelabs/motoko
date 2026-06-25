// Effect-check: an encoder field whose value carries an Await effect
// (here via an embedded async-block inside a do-block) is rejected
// because parenthetical fields must be effect-free.
persistent actor {
  (with encoder = do { ignore (async ()); func () : Blob = "" })
  public func go() : async () {};
}
