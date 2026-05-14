// Negative checks for the self-actor worker `await*` exception.
//
// The exception (typing.ml, AwaitE rule) only greenlights `await*` when the
// callee's `func_sort` is physically equal to one of the enclosing actor's
// canonical `Shared <sort>` cells (env.self_shared). These cases all miss
// that test for different reasons and must still raise M0088.

actor {
  // (a) Private/local async helper — `func_sort` is `T.Local`, not `Shared _`.
  func helper() : async Int { 42 };

  // (b) Reified async value let-bound away from a CallE.
  public func reified() : async () {
    let p : async Int = helper();
    let _n = await* p; // reject: inner expression is `VarE p`, not `CallE _`
  };

  // (c) Direct `await*` on a private (Local) async call.
  public func direct() : async () {
    let _n = await* helper(); // reject: callee is Local, no canonical match
  };
}
