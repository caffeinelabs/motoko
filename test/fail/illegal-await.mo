// immediate deadlock
do {
  let t : async () = async { await t; };
};

// circular deadlock

do {
  ignore async {
    let a1 : async () = async { await a2; }; // illegal await since a1 : Async<X>() </: Async<Y>()
    let a2 : async () = async { await a1; }; // illegal await since a2 : Async<X>() </: Async<Z>()
    ();
  };
};

// Imperative deadlock

do {
  ignore async {
    var x = async { 0 };
    x := (async {
      await x // illegal: await _ : async<S>T -> T (not async<R> T -> T) (any T))
    });
  }
};

// Recursive deadlock

func Rec(n : Int, a : async ()) : async () {
  if n == 0 {
    await a // <- illegal await since async<@>() </: async<X>()
  }
  else {
    await Rec(n - 1, a)
  }
};

do {
  ignore async {
    let t : async () = Rec(10, t);
    await t;
  }
};
