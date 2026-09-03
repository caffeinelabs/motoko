// Showcase: every stable-sig dedup behavior in one minimal example.
//
// [Event] is declared in both modules with equal bodies -> one decl.
// [OldState]/[NewState] are structurally equal under different names
// -> one body decl plus a residual alias [type NewState = OldState]
// preserving the lost name. [Registry] is a parameterized bare alias
// [Map<Text, Event>] declared twice -> one decl (bare aliases group
// unless nullary, where collapsing would print [type X = X]).
// [Tag] is a nullary bare alias [= Text] declared twice -> one decl:
// nullary aliases dedup in a second pass keyed by name and final
// target, so they never collapse onto each other or hijack a prim.
// Re-export aliases whose final target has the same name add nothing
// and vanish entirely: the actor-local [type Event = A.Event] below,
// and [B.Nat], which chases through [A.MyNat] to prim [Nat] so its
// uses print as plain [Nat]. [MyNat] renames its target, so it stays.
//
// The deduped signature is checked in as a static golden at
// test/cmp/files/stable-sig-dedup-showcase.most (regenerate with:
//   moc --stable-types stable-sig-dedup-showcase.mo -o /tmp/x.wasm
//   cp /tmp/x.most ../cmp/files/stable-sig-dedup-showcase.most).
import A "stable-sig-dedup-showcase/a";
import B "stable-sig-dedup-showcase/b";
persistent actor {
  type Event = A.Event;
  let _ea : ?A.Event = null;
  let _eb : ?B.Event = null;
  let _ec : ?Event = null;
  let _n : ?B.Nat = null;
  let _mn : ?A.MyNat = null;
  let _old : ?A.OldState = null;
  let _new : ?B.NewState = null;
  let _ra : ?A.Registry = null;
  let _rb : ?B.Registry = null;
  let _ta : ?A.Tag = null;
  let _tb : ?B.Tag = null;
};
