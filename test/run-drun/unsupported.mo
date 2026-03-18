//MOC-FLAG -A=M0194
// top-level actor objects are supported
actor Counter {

    flexible shared func bad_private_shared() : () { }; // unsupported private shared

    public func ok_actorarg(a:actor{}) : async () {};

    public func ok_functionarg(f:shared()->async ()) : async () {};

    public func ok_oneway() : (){}; // supported oneway

    public func ok() : async () {};

    public func ok_explicit() : async () = async {};

    public func ok_call() : async () {
        ignore (ok()); // supported intercanister messaging
    };

    public func ok_await_call() : async () {
        await (ok()); // supported intercanister messaging
    };

    public func ok_await() : async () {
        let t : async () = loop {};
	await t; // supported general await
    };

    public func ok_async() : async () {
        let a = async { 1; }; // supported async
    };

    public func misc_unsupported () : async () {

        do {
          shared func bad_shared() : () { }; // unsupported non actor-member
        };

        do {
            // shared function types are sharable
            type wellformed_1 = shared (shared () -> ()) -> async ();
        };

        do {
            // actors are shareable
            type wellformed_2 = shared (actor {}) -> async ();
        };

        do {
          actor class BadActorClass () { }; // no actor classes
        };

        do {
          actor class BadActorClass (x : Int) { }; // no actor classes
        };

        do {
         let bad_non_top_actor : actor {} = if true actor {} else actor {};
        };

        do {
          let bad_nested_actor = do { let _ = actor {}; ()};
        };

        do {
          // async functions not supported (inference mode)
          func implicit_async() : async () { };
        };

        do {
          // anonymous shared functions not supported (inference and checking mode)
          let _ = shared func() : async () { };
          ignore (shared func() : async () { }) : shared () -> async ();

        };
    }
};
