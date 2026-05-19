// M10 class form: `class MkRec(args) : Rec = {...}` where the
// declared return type carries top-level `type X` existentials.
//
// The class body is inferred at a concrete type ({value : Nat;
// toText : Nat -> Text}); the declared return type's body is the
// existential pack ({value : X; toText : X -> Text}). The M0134
// body-vs-return-type check has to run witness inference to
// discover X = Nat before the sub-check; callers receive the
// existential pack (Rec), so X stays abstract outside the class.

type Rec = type X in { value : X; toText : X -> Text };

class MkRec(v : Nat) : Rec = {
  public let value = v;
  public let toText = func (n : Nat) : Text = debug_show n;
};

let r : Rec = MkRec 7;

// Destructure to bring the skolem into scope; the witness is
// reachable only via the dedicated open.
let { value; toText } = r;
assert toText value == "7";
