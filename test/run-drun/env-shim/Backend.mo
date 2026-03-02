import { envVar; encodeUtf8; principalOfBlob; actorOfPrincipal; trap } = "mo:⛔";

module {
  type BackendActor = from_candid "service : { increment : () -> (); get : () -> (nat) query }";

  public func Backend<system>() : BackendActor {
    switch (envVar<system>("backend")) {
      case (?backendText) {
        let backendBlob = encodeUtf8(backendText);
        let backendPrincipal = principalOfBlob(backendBlob);
        actorOfPrincipal<BackendActor>(backendPrincipal)
      };
      case null {
        trap("backend envvar not set")
      }
    }
  }
}
