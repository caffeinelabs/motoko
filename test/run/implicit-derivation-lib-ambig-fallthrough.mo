// Ambiguous lib-derivation is no longer terminal: it falls through to structural
// synthesis. Here `mo:amb/AmbigA.show` and `mo:amb/AmbigB.show` both derive
// `show : R -> Text`, making the lib-derivation tier ambiguous; the structural
// `__record` combiner uniquely resolves the hole instead, producing "{a:1,b:2}".
//MOC-FLAG --package amb lib-ambig-stub/src --implicit-package amb

type R = { a : Nat; b : Nat };

// Leaf: resolves the inner `show : Nat -> Text` used by the structural combiner.
func show(n : Nat) : Text = debug_show(n);

// Structural-record combiner: unique candidate at the structural tier.
module Combiner {
  public func show(__record : [(Text, () -> Text)]) : Text {
    var s = "{";
    var first = true;
    for ((k, v) in __record.vals()) {
      if (not first) { s #= "," };
      s #= k # ":" # v();
      first := false;
    };
    s # "}";
  };
};

func render(r : R, show : (implicit : R -> Text)) : Text = show(r);

assert render({ a = 1; b = 2 }) == "{a:1,b:2}";
