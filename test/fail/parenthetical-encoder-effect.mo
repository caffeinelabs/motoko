// An encoder whose value carries an `await` effect (via an embedded `async`
// block) is rejected: codec fields must be effect-free (M0279).
actor {
  public (with encoder = do { ignore (async ()); func () : Blob = "" }) func go() : async () {};
}
