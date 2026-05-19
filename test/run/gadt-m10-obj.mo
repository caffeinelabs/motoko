// M10 records / `object` form: construct an existentially-typed record
// via `object { public let ...; public func ... }` syntax. The
// existential is packed at construction, same as for record-literal
// syntax. Destructuring re-introduces the skolem.

type Rec = type X in { value : X; render : X -> Text };

func natToText(n : Nat) : Text = debug_show n;

let r : Rec = object {
  public let value = 5;
  public let render = natToText;
};

let { value = v; render = f } = r;
assert f v == "5";
