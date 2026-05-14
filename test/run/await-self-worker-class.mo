// Exercises all three branches of the syntactic AwaitE classifier on an
// `actor class` with two instances:
//   - the first instance receives `null` as its prior-sibling argument;
//   - the second receives `?first`.
// Inside `go()`:
//   - `await* foo()`        — Obvious self via unqualified VarE
//                             (foo ∈ env.public_methods);
//   - `await* self.foo()`   — Obvious self via DotE on env.self_id;
//   - `await* s.foo()`      — Maybe self via DotE on an actor-typed
//                             receiver (s : A, unwrapped from `prior`).
// AST interpreter runs all three via the polymorphic `await*` (Async
// fallback to plain `await`); worker fast-path is a phase-2 lowering
// concern that doesn't affect observable semantics here.

persistent actor class A(prior : ?A) = self {
  public func foo() : async Nat { switch prior { case (?_) 42; case _ 21 } };

  public func go() : async Nat {
    let n_unqual = await* foo();
    let n_self   = await* self.foo();
    switch prior {
      case (?s) {
        let n_other = await* s.foo();
        n_unqual + n_self + n_other
      };
      case null { n_unqual + n_self };
    }
  };
};

let first = await A(null);
let second = await A(?first);
let total = await second.go();
// second.foo() = 42 (prior = ?first matches `?_`) called twice (Obvious self
// via VarE and via DotE self), plus first.foo() = 21 (prior = null matches
// the wildcard) via Maybe-self DotE: 42 + 42 + 21 = 105.
assert total == 105;

//SKIP comp
