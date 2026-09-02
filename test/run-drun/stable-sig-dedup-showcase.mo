// Showcase: every stable-sig dedup behavior in one minimal example.
//
// [Event] is declared in both modules with equal bodies -> one decl.
// [OldState]/[NewState] are structurally equal under different names
// -> one body decl plus a residual alias [type NewState = OldState]
// preserving the lost name. [Registry] is a parameterized bare alias
// [Map<Text, Event>] declared twice -> one decl (bare aliases group
// unless nullary, where collapsing would print [type X = X]).
//
// The deduped signature is checked in as a static golden at
// test/cmp/files/stable-sig-dedup-showcase.most (regenerate with:
//   moc --stable-types stable-sig-dedup-showcase.mo -o /tmp/x.wasm
//   cp /tmp/x.most ../cmp/files/stable-sig-dedup-showcase.most).
import A "stable-sig-dedup-showcase/a";
import B "stable-sig-dedup-showcase/b";
persistent actor {
  let _ea : ?A.Event = null;
  let _eb : ?B.Event = null;
  let _old : ?A.OldState = null;
  let _new : ?B.NewState = null;
  let _ra : ?A.Registry = null;
  let _rb : ?B.Registry = null;
};
