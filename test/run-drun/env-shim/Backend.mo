import { envVar; encodeUtf8; principalOfBlob; actorOfPrincipal; trap } = "mo:⛔";

module {
  type BackendActor = from_candid "service : { increment : () -> (); get : () -> (nat) query }";

  public func Backend<system>() : BackendActor {
    switch (envVar<system> "backend") {
      case (?backendText) {
        backendText
          |> encodeUtf8 _
          |> principalOfBlob _
          |> actorOfPrincipal<BackendActor> _
      };
      case null {
        trap("backend envvar not set")
      }
    }
  }
}
