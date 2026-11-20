func app<T>(f: T -> T, x : T) : T { f(x) };

let r : Int = app(func j = -1, 1);

// A return type annotation must fit the expected return type that the call fixes
func apply<T>(x : T, f : T -> Nat) : Nat = f(x);
func applyText<T>(x : T, f : T -> Text) : Text = f(x);
func applyAsync<T>(x : T, f : T -> async Nat) : async Nat { await f(x) };
func applyAsyncStar<T>(x : T, f : T -> async* Nat) : async* Nat { await* f(x) };

func _m1() {
  let _ = apply(5, func(_) : Int { -1 }) // a supertype
};
func _m2() {
  let _ = applyText(5, func(x) : Nat { x }) // an unrelated type
};
func _m3() : async () {
  let _ = await applyAsync(5, func(_) : async Int { -1 })
};
func _m4() : async* () {
  let _ = await* applyAsyncStar(5, func(_) : async* Int { -1 })
};

module Dot {
  public func apply<T>(self : T, f : T -> Nat) : Nat = f(self)
};
func _m5() {
  let _ = (5).apply(func(_) : Int { -1 }) // a contextual dot call
}
