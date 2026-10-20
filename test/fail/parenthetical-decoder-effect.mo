// A decoder whose value carries an `await` effect (via an embedded `async`
// block) is rejected: codec fields must be effect-free (M0279).
actor {
  public (with decoder = do { ignore (async ()); func (_ : Blob) : ?() = ?() }) func go(_ : ?()) : async () {};
}
