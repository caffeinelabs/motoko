// Variant-arm existential cross-mixing: sibling case arms over an
// existential-bearing variant. Without per-arm fresh skolems, both
// bind at a shared schema cons → cross-feed `f1 x2` (Nat→Text
// applied to a Bool) silently type-checks and runtime-traps. With
// M11b path B (fresh_destructure_skolem per pat.at + σ-substitute
// in the destructure'd arm payload), the two arms bind at distinct
// cons and the application is statically rejected.

type Box = {
  #pack : type X in (X, X -> Text);
};

let box1 : Box = #pack (42 : Nat, func (n : Nat) : Text = debug_show n);
let box2 : Box = #pack (true, func (b : Bool) : Text = debug_show b);

func dangerous(b1 : Box, b2 : Box) : Text =
  switch b1 {
    case (#pack (_, f1))
      switch b2 {
        case (#pack (x2, _))
          f1 x2
      }
  };

let _result = dangerous(box1, box2);
