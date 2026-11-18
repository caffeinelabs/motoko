// An encoder whose value carries an `await` effect (via an embedded `async`
// block) is rejected: codec fields must be effect-free (M0279).
actor {
  (with encoder = do { ignore (async ()); func () : Blob = "" })
  public func go() : async () {};
}
