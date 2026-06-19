func describe(__tuple : [() -> Text]) : Text = "(...)";
module TextDesc { public func describe(self : Text) : Text = self };

func inspect<T>(x : T, describe : (implicit : T -> Text)) : Text = describe(x);

ignore inspect(("hello", true));
