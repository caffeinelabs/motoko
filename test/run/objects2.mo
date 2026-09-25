type O = { self : () -> O; x : Nat };
let o = object this { public func self() : O = this; public let x = 1 };
assert(o.x == 1);
assert(o.self().x == 1);
assert(o.self().self().x == 1);
