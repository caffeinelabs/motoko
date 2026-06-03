// Structural derivation fails when a nested record hits the depth limit.
// { inner : { x : Text } } requires two levels of structural synthesis
// (depth 0 for the outer, depth 1 for the inner), but the limit is 1.
//MOC-FLAG --implicit-derivation-depth 1

type Tag = { #str : Text; #obj : [(Text, Tag)] };

// Structural combiner (search label: `enc`)
module TagRec { public func enc(__record : [(Text, () -> Tag)]) : Tag = #obj([]) };
// Per-type instance for Text
module TagText { public func enc(self : Text) : Tag = #str self };

func encode<R>(x : R, enc : (implicit : R -> Tag)) : Tag = enc(x);

// Outer struct derivation at depth 0 succeeds;
// inner { x : Text } derivation at depth 1 is blocked by the limit.
ignore encode({ inner = { x = "hi" } });
