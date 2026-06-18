//MOC-FLAG --actor-idl actor-funcparam-destructured
//MOC-FLAG --actor-alias self rwlgt-iiaaa-aaaaa-aaaaa-cai

// gabor/actor-method-destructured:
//   FuncE / ClassD parameter pre-massage — ObjP-against-actor params
//   project per-field via ActorDotPrim, sidestepping the I.ObjP route.
//   Passes `this` (the test actor itself, narrowed to its public
//   surface) into each callee and exercises the destructured handles.

import _IC "canister:self";

actor a {
  public func ping() : async () { };
  public func pong() : async () { };

  type Self = actor { ping : () -> async (); pong : () -> async () };

  // (1) single ObjP field
  func f1({ ping } : Self) : async () {
    await ping()
  };

  // (2) multiple ObjP fields
  func f2({ ping; pong } : Self) : async () {
    await ping();
    await pong()
  };

  // (3) AnnotP wrapping the ObjP
  func f3(({ ping } : Self)) : async () {
    await ping()
  };

  // (4) ParP wrapping the ObjP (nested parens)
  func f4((({ ping }) : Self)) : async () {
    await ping()
  };

  // (5) AndP: ObjP and a whole-actor name
  func f5(({ ping } and whole) : Self) : async () {
    await ping();
    await whole.pong()
  };

  // (5b) AndP, swapped arms: whole-actor name and ObjP (ObjP on the right)
  func f5b((whole and { pong }) : Self) : async () {
    await whole.ping();
    await pong()
  };

  // (5c) AndP of two ObjPs against the same actor
  func f5c(({ ping } and { pong }) : Self) : async () {
    await ping();
    await pong()
  };

  // (5d) chained AndP with a ParP-wrapped ObjP arm — stresses wrapper-peeling
  //   across `and` (whole-name + ObjP + parenthesised ObjP).
  func f5d((whole and { pong } and ({ ping })) : Self) : async () {
    await whole.pong();
    await pong();
    await ping()
  };

  // (6) shared func — exercises the must_wrap=true path in to_args
  public shared func f6_shared({ ping } : Self) : async () {
    await ping()
  };

  // (7) ClassD param — same to_args entry point
  class Observer({ ping; pong } : Self) {
    public func tick() : async () {
      await ping();
      await pong()
    };
  };

  // (8) ForE iter pattern — actor element type.
  //   The iter expression itself is `{ next : () -> ?T }` (a record;
  //   can't be actor — actor methods must be async).  But T (the
  //   element type) can be actor.  The loop pattern matches against
  //   T, so ObjP-against-actor is legitimately reachable here.
  func f8_for_loop(it : { next : () -> ?Self }) : async () {
    for ({ ping } in it) {
      await ping()
    }
  };

  // (9) let-else with an actor ObjP. The pattern is IRREFUTABLE (object
  //   destructure of a known-shape actor always succeeds), so the `else` is
  //   dead and there should be a single projecting match, not a refutable
  //   cascade — and it must take the ActorDotPrim path, not record offsets.
  func f9_let_else(self : Self) : async () {
    let { pong } = self else { return };
    await pong()
  };

  public func test() : async () {
    // let-else first, then the AndP family, to isolate each.
    await f9_let_else(a);
    await f5(a);
    await f5b(a);
    await f5c(a);
    await f5d(a);
    await f1(a);
    await f2(a);
    await f3(a);
    await f4(a);
    await f6_shared(a);
    let o = Observer(a);
    await o.tick();
    // ForE with an iter that yields `a` once, then null.
    let yielded = { var done_ = false };
    let single_iter : { next : () -> ?Self } = {
      next = func() : ?Self {
        if (yielded.done_) null else { yielded.done_ := true; ?a }
      }
    };
    await f8_for_loop(single_iter);
  };
};

// Actually invoke `test()` under drun so the destructure paths (esp. the AndP
// f5/f5b/f5c) execute — otherwise the actor only installs and a runtime trap
// (e.g. ObjP-against-actor mis-lowered to record offsets) goes uncaught.
//CALL ingress test "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
