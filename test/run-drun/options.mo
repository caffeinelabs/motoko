import Prim "mo:prim";

// Test (type optimized) option injection and projection works correctly

// Basically the same test repeated at different,
// representative type instantiations (staticially unboxed, statically mixed,
// statically unknown)
// Intended to test compile_enhanced/classical/Opt.injection_is_free type-based
// option optimization

actor {

  // Any
  do {
    type T = Any;
    let v : T = #any;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // Null
  do {
    type T = Null;
    let v : T = null;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // Bool true
  do {
    type T = Bool;
    let v : T = true; // true has special representation

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // Bool false
  do {
    type T = Bool;
    let v : T = false;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // Nat8
  do {
    type T = Nat8;
    let v : T = 0;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // Nat64 (always boxed)
  do {
    type T = Nat64;
    let v : T = 0;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // Nat (unboxed)
  do {
    type T = Nat;
    let v : T = 0;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // Nat (boxed)
  do {
    type T = Nat;
    let v : T = 18446744073709551616; //2^^64

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // variant
  do {
    type T = {#lab};
    let v : T = #lab;

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // record
  do {
    type T = { fld : ()};
    let v : T = {fld = ()};

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // array
  do {
    type T = [Any];
    let v : T = [];

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // unit
  do {
    type T = ();
    let (#unit v) = #unit ();  // avoids MO239 warning

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };

  // pair
  do {
    type T = (Nat, Bool);
    let v : T = (0, true);

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and w == v; };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and w == v };
        };
        i += 1;
      };
    };
  };


  // errors
  do {
    type T = Error;
    let v : T = Prim.error("some error");

    do {
      var i = 0;
      for(o in [null, ?v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?w) { assert i == 1 and
	    Prim.errorMessage(w) == Prim.errorMessage(v);
	  };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??w) { assert i == 2 and
  	    Prim.errorMessage(w) == Prim.errorMessage(v);
	  };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null, ???v].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
          case (???w) { assert i == 3 and
	    Prim.errorMessage(w) == Prim.errorMessage(v);
          };
        };
        i += 1;
      };
    };
  };


  func generic_test<T>(v : T, eq : (T, T) -> Bool) {
    do {

      do {
	var i = 0;
	for(o in [null, ?v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?w) { assert i == 1 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??w) { assert i == 2 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??null, ???v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??null) { assert i == 2 };
	    case (???w) { assert i == 3 and eq(w, v) };
	  };
	  i += 1;
	};
      };
    };
  };

  generic_test<Any>("", func (t1, t2) = t1 == t2);
  generic_test<Text>("", func (t1, t2) = t1 == t2);
  generic_test<Nat8>(0, func (t1, t2) = t1 == t2);
  generic_test<Nat64>(0, func (t1, t2) = t1 == t2);
  generic_test<Nat>(0, func (t1, t2) = t1 == t2);
  generic_test<Null>(null, func (t1, t2) = t1 == t2);
  generic_test<()>((), func (t1, t2) = t1 == t2);
  generic_test<?()>(?(), func (t1, t2) = t1 == t2);


  func bounded_test<T <: Int>(v : T, eq : (T, T) -> Bool) {
    do {

      do {
	var i = 0;
	for(o in [null, ?v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?w) { assert i == 1 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??w) { assert i == 2 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??null, ???v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??null) { assert i == 2 };
	    case (???w) { assert i == 3 and eq(w, v) };
	  };
	  i += 1;
	};
      };
    };
  };

  bounded_test<Int>(-1, func (t1, t2) = t1 == t2);
  bounded_test<Nat>(0, func (t1, t2) = t1 == t2);


  func rec_bounded_test<T <: U, U>(v : T, eq : (T, T) -> Bool) {
    do {

      do {
	var i = 0;
	for(o in [null, ?v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?w) { assert i == 1 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??w) { assert i == 2 and eq(w, v); };
	  };
	  i += 1;
	}
      };

      do {
	var i = 0;
	for(o in [null, ?null, ??null, ???v].values()) {
	  switch o {
	    case null { assert i == 0 };
	    case (?null) { assert i == 1 };
	    case (??null) { assert i == 2 };
	    case (???w) { assert i == 3 and eq(w, v) };
	  };
	  i += 1;
	};
      };
    };
  };

  rec_bounded_test<Int, Int>(-1, func (t1, t2) = t1 == t2);
  rec_bounded_test<Nat, Int>(0, func (t1, t2) = t1 == t2);


  // None
  do {
    type T = None;

    do {
      var i = 0;
      for(o in [null].values()) {
        switch o {
          case null { assert i == 0 };
//        case (?_w) { assert false };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
//        case (??w) { assert false };
        };
        i += 1;
      }
    };

    do {
      var i = 0;
      for(o in [null, ?null, ??null].values()) {
        switch o {
          case null { assert i == 0 };
          case (?null) { assert i == 1 };
          case (??null) { assert i == 2 };
//        case (???_w) { assert false };
        };
        i += 1;
      };
    };
  };

};


