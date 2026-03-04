// simplified version of data-view.mo that doesn't require core.
//MOC-FLAG --generate-view-queries
import Prim "mo:⛔";
import Client "data-view-admin/client";
persistent actor Self {

  // a stable variable with viewer __view : () -> async [Nat]
  let view = [1,2,3];

  transient var admin : ?Principal = null;

  // isAdmin, when declared, is used to control access to generated views
  func isAdmin(caller : Principal) : Bool {
    (?caller) == admin
  };

  public func go() : async () {

    let server = actor (debug_show (Prim.principalOfActor(Self))) :
      actor {
        __view : query () -> async [Nat];
     };

    let client = await Client.Client(server);

    // call __view from client without admin rights (fails)
    try {
      Prim.debugPrint(debug_show (await client.test()));
    } catch e {
      Prim.debugPrint(debug_show (Prim.errorMessage(e)));
    };

    // set c1 as admin
    admin := ?Prim.principalOfActor(client);

    // call __view from client with admin rights (succeeds)
    try {
      Prim.debugPrint(debug_show (await client.test()));
    } catch e {
      assert(false);
    };
  }
}
//CALL ingress go "DIDL\x00\x00"
