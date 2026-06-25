// Effect-check (decoder side): a decoder field whose value carries
// an Await effect (here via an embedded async-block inside a do-
// block) is rejected because parenthetical fields must be effect-free.
persistent actor {
  (with decoder = do { ignore (async ()); func (_ : Blob) : ?() = ?() })
  public func go(_ : ?()) : async () {};
}
