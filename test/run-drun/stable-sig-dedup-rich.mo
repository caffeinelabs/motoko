//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors --enhanced-migration stable-sig-dedup-rich/migrations
//SKIP run
//SKIP run-ir
//SKIP run-low
// Showcase: a realistic 3-step migration chain modelled on a "rich" app
// (long-rich/ in the enhanced-migrations stress test). Each migration
// module redeclares `Event`/`User`/`Order` locally, so without
// stable-sig dedup the emitted `.most` carries 4 copies of each
// (one per module + one from the actor). Dedup collapses them to 1.
//
// The deduped stable-type signature is checked in as a static golden at
// test/cmp/files/stable-sig-dedup-rich.most (regenerate with:
//   moc --enhanced-orthogonal-persistence --default-persistent-actors \
//       --enhanced-migration stable-sig-dedup-rich/migrations \
//       --stable-types stable-sig-dedup-rich.mo -o /tmp/x.wasm
//   cp /tmp/x.most ../cmp/files/stable-sig-dedup-rich.most).

actor {
  type Item = { id : Nat; name : Text; tag : Text; count : Nat };
  type User = { id : Nat; email : Text; active : Bool };
  type Order = { id : Nat; amount : Nat; status : Text };
  type Event = { #added : Nat; #removed : Nat };

  let items : [Item];
  let users : [User];
  let orders : [Order];
  let events : [Event];

  public query func sizes() : async (Nat, Nat, Nat, Nat) {
    (items.size(), users.size(), orders.size(), events.size())
  };
};
