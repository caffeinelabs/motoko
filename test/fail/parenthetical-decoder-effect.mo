// A decoder whose value carries an `await` effect (via an embedded `async`
// block) is rejected: codec fields must be effect-free (M0279).
actor {
  (with decoder = do { ignore (async ()); func (_ : Blob) : ?() = ?() })
  public func go(_ : ?()) : async () {};
}
