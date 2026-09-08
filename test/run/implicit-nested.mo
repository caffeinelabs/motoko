//MOC-FLAG --package core $MOTOKO_CORE

type Monoid<T> = module {
  empty : () -> T;
  combine : (T, T) -> T;
};

func fold<T>(xs : [T], Monoid : (implicit : Monoid<T>)) : T {
  var acc = Monoid.empty();
  for (x in xs.vals()) {
    acc := Monoid.combine(acc, x);
  };
  acc;
};

module Monoids {
  public module Text {
    public module Monoid {
      public func empty() : Text = "";
      public func combine(a : Text, b : Text) : Text { a # b };
    };
  };
  public module One {
    public module Two {
      public module Three {
        public module Four {
          public module Five {
           public module Six {
             public module Seven {
               // Maximum search depth for nested modules
               public module Monoid {
                 public func empty() : Nat = 0;
                 public func combine(a : Nat, b : Nat) : Nat { a + b };
               };
               public module Eight {
                 public module Monoid {
                   public func empty() : Int = 0;
                   public func combine(a : Int, b : Int) : Int { a + b };
                 };
               }
             }
           }
          }
        }
      }
    }
  };
};

let s = fold(["hello", " ", "world"]);
let n = fold([1, 2, 3]);
assert s == "hello world";
assert n == 6;

// let i = fold([-1, 2, -3]);
