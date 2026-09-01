// Variant structural derivation fails when a case payload has no instance.
func describe(__variant : (Text, () -> Text)) : Text {
  let (tag, payload) = __variant;
  tag # ":" # payload();
};
module TextDesc { public func describe(self : Text) : Text = self };

func inspect<T>(x : T, describe : (implicit : T -> Text)) : Text = describe(x);

// `#flag : Bool` has no `describe : Bool -> Text` instance → derivation fails.
ignore inspect(#flag true);
