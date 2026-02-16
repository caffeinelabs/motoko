//MOC-FLAG -A=M0194
func bot() : None { bot() };

// Any
let _ : [var () -> Any] = do { let x =  // use block to force inference mode
  [ func() : Int { bot() },
    func() : Any { bot() },
  ]; [var x[0]]  // use var array to force invariance
};
let _ : [var Nat -> ()] = do { let x =
  [ func(_ : Any) {},
    func(_ : Nat) {},
  ]; [var x[0]]
};

// None
let _ : [var () -> Int] = do { let x =
  [ func() : Int { bot() },
    func() : None { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : None) {},
    func(_ : Nat) {},
  ]; [var x[0]]
};

// Primitives
let _ : [var () -> Int] = do { let x =
  [ func() : Int { bot() },
    func() : Nat { bot() },
  ]; [var x[0]]
};
let _ : [var Nat -> ()] = do { let x =
  [ func(_ : Int) {},
    func(_ : Nat) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : Int { bot() },
    func() : Bool { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : Text) {},
    func(_ : Nat) {},
  ]; [var x[0]]
};

// Options
let _ : [var () -> ?Int] = do { let x =
  [ func() : ?Int { bot() },
    func() : ?Nat { bot() },
  ]; [var x[0]]
};
let _ : [var ?Nat -> ()] = do { let x =
  [ func(_ : ?Int) {},
    func(_ : ?Nat) {},
  ]; [var x[0]]
};
let _ : [var () -> ?Nat] = do { let x =
  [ func() : Null { bot() },
    func() : ?Nat { bot() },
  ]; [var x[0]]
};
let _ : [var Null -> ()] = do { let x =
  [ func(_ : ?Nat) {},
    func(_ : Null) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : Nat { bot() },
    func() : ?Nat { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : ?Nat) {},
    func(_ : Nat) {},
  ]; [var x[0]]
};

// Arrays
let _ : [var () -> [Int]] = do { let x =
  [ func() : [Int] { bot() },
    func() : [Nat] { bot() },
  ]; [var x[0]]
};
let _ : [var [Nat] -> ()] = do { let x =
  [ func(_ : [Int]) {},
    func(_ : [Nat]) {},
  ]; [var x[0]]
};
let _ : [var () -> [var Nat]] = do { let x =
  [ func() : [var Nat] { bot() },
    func() : [var Nat] { bot() },
  ]; [var x[0]]
};
let _ : [var [var Nat] -> ()] = do { let x =
  [ func(_ : [var Nat]) {},
    func(_ : [var Nat]) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : [var Nat] { bot() },
    func() : [var Int] { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : [var Nat]) {},
    func(_ : [var Int]) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : [Nat] { bot() },
    func() : [var Nat] { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : [Nat]) {},
    func(_ : [var Nat]) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : Nat { bot() },
    func() : [Nat] { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : Nat) {},
    func(_ : [Nat]) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : Nat { bot() },
    func() : [var Nat] { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : Nat) {},
    func(_ : [var Nat]) {},
  ]; [var x[0]]
};

// Tuples
let _ : [var () -> (Int, Int)] = do { let x =
  [ func() : (Nat, Int) { bot() },
    func() : (Int, Nat) { bot() },
  ]; [var x[0]]
};
let _ : [var ((Nat, Nat)) -> ()] = do { let x =
  [ func(_ : (Nat, Int)) {},
    func(_ : (Int, Nat)) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : Nat { bot() },
    func() : ((Nat, Nat)) { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : Nat) {},
    func(_ : (Nat, Nat)) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : ((Nat, Nat)) { bot() },
    func() : ((Nat, Nat, Nat)) { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : (Nat, Nat)) {},
    func(_ : (Nat, Nat, Nat)) {},
  ]; [var x[0]]
};

// Objects
let _ : [var () -> {a : Int}] = do { let x =
  [ func() : {a : Nat; b : Nat} { bot() },
    func() : {a : Int; var c : Nat} { bot() },
  ]; [var x[0]]
};
let _ : [var {a : Nat; b : Nat; var c : Nat} -> ()] = do { let x =
  [ func(_ : {a : Nat; b : Nat}) {},
    func(_ : {a : Int; var c : Nat}) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : {a : Nat; b : Nat} { bot() },
    func() : {a : Nat; var b : Nat} { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : {a : Nat; b : Nat}) {},
    func(_ : {a : Nat; var b : Nat}) {},
  ]; [var x[0]]
};

// Variants
let _ : [var () -> {#a : Int; #b : Nat; #c : Nat}] = do { let x =
  [ func() : {#a : Nat; #b : Nat} { bot() },
    func() : {#a : Int; #c : Nat} { bot() },
  ]; [var x[0]]
};
let _ : [var {#a : Nat} -> ()] = do { let x =
  [ func(_ : {#a : Nat; #b : Nat}) {},
    func(_ : {#a : Int; #c : Nat}) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : {a : Nat} { bot() },
    func() : {#a : Nat} { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : {a : Nat}) {},
    func(_ : {#a : Nat}) {},
  ]; [var x[0]]
};

// Functions
let _ : [var () -> (<A, B <: A>(Nat, B) -> (Int, A))] = do { let x =
  [ func() : <A, B <: A>(Nat, A) -> (Int, A) { bot() },
    func() : <A, B <: A>(Int, B) -> (Nat, B) { bot() },
  ]; [var x[0]]
};
let _ : [var (<A, B <: A>(Int, A) -> (Nat, B)) -> ()] = do { let x =
  [ func(_ : <A, B <: A>(Nat, A) -> (Int, A)) {},
    func(_ : <A, B <: A>(Int, B) -> (Nat, B)) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : <A <: Nat>() -> () { bot() },
    func() : <A <: Int>() -> () { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : <A <: Int>() -> ()) {},
    func(_ : <A <: Nat>() -> ()) {},
  ]; [var x[0]]
};
let _ : [var () -> Any] = do { let x =
  [ func() : <A>() -> () { bot() },
    func() : <A, B>() -> () { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : <A>() -> ()) {},
    func(_ : <A, B>() -> ()) {},
  ]; [var x[0]]
};

// Abstract types
func f<A <: {a : Nat}, B <: {a : Int; b : Nat}, C <: A>() {
  let _ : [var () -> {a : Int}] = do { let x =
    [ func() : A { bot() },
      func() : B { bot() },
    ]; [var x[0]]
  };
  let _ : [var None -> ()] = do { let x =
    [ func(_ : A) {},
      func(_ : B) {},
    ]; [var x[0]]
  };

  let _ : [var () -> A] = do { let x =
    [ func() : A { bot() },
      func() : C { bot() },
    ]; [var x[0]]
  };
  let _ : [var C -> ()] = do { let x =
    [ func(_ : A) {},
      func(_ : C) {},
    ]; [var x[0]]
  };

  let _ : [var () -> {a : Int}] = do { let x =
    [ func() : B { bot() },
      func() : C { bot() },
    ]; [var x[0]]
  };
  let _ : [var None -> ()] = do { let x =
    [ func(_ : B) {},
      func(_ : C) {},
    ]; [var x[0]]
  };

  let _ : [var () -> {}] = do { let x =
    [ func() : A { bot() },
      func() : {b : Nat} { bot() },
    ]; [var x[0]]
  };
  let _ : [var None -> ()] = do { let x =
    [ func(_ : A) {},
      func(_ : {b : Nat}) {},
    ]; [var x[0]]
  };
  let _ : [var () -> Any] = do { let x =
    [ func() : A { bot() },
      func() : Nat { bot() },
    ]; [var x[0]]
  };
  let _ : [var None -> ()] = do { let x =
    [ func(_ : A) {},
      func(_ : Nat) {},
    ]; [var x[0]]
  };
};

// Recursive types
type A = {b : B};
type B = {a : A};
type C = {b : {a : C; c : C}};

let _ : [var () -> A] = do { let x =
  [ func() : A { bot() },
    func() : C { bot() },
  ]; [var x[0]]
};
let _ : [var C -> ()] = do { let x =
  [ func(_ : A) {},
    func(_ : C) {},
  ]; [var x[0]]
};

type D = {x : {x : D; y : Int}};
type E = {x : {x : {x : E}; y : Nat}; z : Int};
type lubDE =
  { x :
    { x :
      { x :
        { x :
          { x :
            { x : lubDE;
            };
          };
        };
      };
      y : Int;
    };
  };
type glbDE =
  { x :
    { x :
      { x :
        { x :
          { x :
            { x : glbDE;
              y : Int;
            };
            y : Nat;
          };
          y : Int;
          z : Int;
        };
      };
      y : Nat;
    };
    z : Int;
  };

let _ : [var () -> lubDE] = do { let x =
  [ func() : D { bot() },
    func() : E { bot() },
  ]; [var x[0]]
};
let _ : [var glbDE -> ()] = do { let x =
  [ func(_ : D) {},
    func(_ : E) {},
  ]; [var x[0]]
};

// Type members
class T() {
  public type T = Nat;
};
class U() {
  public type T = Int;
};

let _ : [var () -> T] = do { let x =
  [ func() : T { bot() },
    func() : T { bot() },
  ]; [var x[0]]
};
let _ : [var U -> ()] = do { let x =
  [ func(_ : U) {},
    func(_ : U) {},
  ]; [var x[0]]
};
let _ : [var () -> {}] = do { let x =
  [ func() : T { bot() },
    func() : U { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : T) {},
    func(_ : U) {},
  ]; [var x[0]]
};
let _ : [var () -> {}] = do { let x =
  [ func() : T { bot() },
    func() : {T : Nat} { bot() },
  ]; [var x[0]]
};
let _ : [var None -> ()] = do { let x =
  [ func(_ : T) {},
    func(_ : {T : Nat}) {},
  ]; [var x[0]]
};


// Intersection
do {
  let _ : [var Int and Nat] = [var] : [var Nat];
  let _ : [var {} and {a : Nat}] = [var] : [var {a : Nat}];
  let _ : [var {a : Nat} and {a : Int}] = [var] : [var {a : Nat}];
  let _ : [var {a : Nat} and {b : Nat}] = [var] : [var {a : Nat; b : Nat}];
  let _ : [var {#} and {#a : Int}] = [var] : [var {#}];
  let _ : [var {#a : Nat} and {#a : Int}] = [var] : [var {#a : Nat}];
  let _ : [var {#a : Nat} and {#b : Nat}] = [var] : [var {#}];
  let _ : [var [Nat] and [Int]] = [var] : [var [Nat]];
  let _ : [var [var Nat] and [var Int]] = [var] : [var None];
  let _ : [var [var Nat] and [Nat]] = [var] : [var None];
  let _ : [var ?Nat and ?Int] = [var] : [var ?Nat];
  let _ : [var () and {}] = [var] : [var None];
  let _ : [var (Nat, Int) and (Int, Nat)] = [var] : [var (Nat, Nat)];
  let _ : <A>[var A and A] -> () = func<A>(x : [var A]) {};
  let _ : <A, B>[var A and B] -> () = func<A, B>(x : [var None]) {};
  let _ : <A, B<:A>[var A and B] -> () = func<A, B<:A>(x : [var B]) {};
  let _ : <A, B<:A, C<:B>[var A and C] -> () = func<A, B<:A, C<:B>(x : [var C]) {};
  let _ : <A<:Nat, B<:Int>[var A and B] -> () = func<A<:Nat, B<:Int>(x : [var None]) {};
  let _ : [var <A, B<:A>(Nat, Int, A, B) -> (Nat, Int, A, B) and <A, B<:A>(Int, Nat, B, A) -> (Int, Nat, B, A)] = [var] : [var <A, B<:A>(Int, Int, A, A) -> (Nat, Nat, B, B)];
  let _ : [var <A<:Nat>() -> () and <A<:Int>() -> ()] = [var] : [var None];
  let _ : [var Nat and Any] = [var] : [var Nat];
  let _ : [var Nat and None] = [var] : [var None];
};

// Union
do {
  let _ : [var Int or Nat] = [var] : [var Int];
  let _ : [var {} or {a : Nat}] = [var] : [var {}];
  let _ : [var {a : Nat} or {a : Int}] = [var] : [var {a : Int}];
  let _ : [var {a : Nat} or {b : Nat}] = [var] : [var {}];
  let _ : [var {#} or {#a : Nat}] = [var] : [var {#a : Nat}];
  let _ : [var {#a : Nat} or {#a : Int}] = [var] : [var {#a : Int}];
  let _ : [var {#a : Nat} or {#b : Nat}] = [var] : [var {#a : Nat; #b : Nat}];
  let _ : [var [Nat] or [Int]] = [var] : [var [Int]];
  let _ : [var [var Nat] or [var Int]] = [var] : [var Any];
  let _ : [var [var Nat] or [Nat]] = [var] : [var Any];
  let _ : [var ?Nat and ?Int] = [var] : [var ?Nat];
  let _ : [var () or {}] = [var] : [var Any];
  let _ : [var (Nat, Int) or (Int, Nat)] = [var] : [var (Int, Int)];
  let _ : <A>[var A or A] -> () = func<A>(x : [var A]) {};
  let _ : <A, B>[var A or B] -> () = func<A, B>(x : [var Any]) {};
  let _ : <A, B<:A>[var A or B] -> () = func<A, B<:A>(x : [var A]) {};
  let _ : <A, B<:A, C<:B>[var A or C] -> () = func<A, B<:A, C<:B>(x : [var A]) {};
  let _ : <A<:Nat, B<:Int>[var A or B] -> () = func<A<:Nat, B<:Int>(x : [var Int]) {};
  let _ : [var <A, B<:A>(Nat, Int, A, B) -> (Nat, Int, A, B) or <A, B<:A>(Int, Nat, B, A) -> (Int, Nat, B, A)] = [var] : [var <A, B<:A>(Nat, Nat, B, B) -> (Int, Int, A, A)];
  let _ : [var <A<:Nat>() -> () or <A<:Int>() -> ()] = [var] : [var Any];
  let _ : [var Nat or Any] = [var] : [var Any];
  let _ : [var Nat or None] = [var] : [var Nat];
};
