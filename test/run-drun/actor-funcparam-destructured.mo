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

  public func test() : async () {
    await f1(a);
    await f2(a);
    await f3(a);
    await f4(a);
    await f5(a);
    await f6_shared(a);
    let o = Observer(a);
    await o.tick();
  };
};

//SKIP run
//SKIP run-ir
//SKIP run-low
