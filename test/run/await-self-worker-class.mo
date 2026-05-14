// Exercises both branches of the syntactic AwaitE classifier on an
// `actor class` with two instances:
//   - the first instance receives `null` as its prior-sibling argument;
//   - the second receives `?first`.
// Inside `go()`:
//   - `await* foo()` is an Obvious self-call (unqualified, foo ∈ public_methods);
//   - `await* s.foo()` (where `s : A` is unwrapped from the optional sibling)
//     is a Maybe-self call (DotE on an actor-typed receiver, static method name).
// AST interpreter runs both via the polymorphic `await*` (Async fallback to
// plain await); the worker fast-path is a phase-2 lowering concern.

persistent actor class A(prior : ?A) {
  public func foo() : async Nat { 42 };

  public func go() : async Nat {
    let n_self = await* foo();
    switch prior {
      case (?s) {
        let n_other = await* s.foo();
        n_self + n_other
      };
      case null { n_self };
    }
  };
};

let first = await A(null);
let second = await A(?first);
let total = await second.go();
assert total == 84;

//SKIP comp
