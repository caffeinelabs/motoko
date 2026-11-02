% Motoko
% A comprehensive overview

# Overview

### Motivation and Goals

A simple, useful language for the Internet Computer (IC)

* Familiar syntax
* Safe by default
* Incorporating ~~actor~~ *canister* model
* Seamless integration of IC features
* Making most of present and future WebAssembly

### Key Design Points

* Object-oriented, functional & imperative
* Objects as records of members
* `async`/`await` for sequential programming of asynchronous messaging
* Structural typing with simple generics and subtyping
* Safe arithmetic (unbounded and checked)
* Non-nullable types by default
* JavaScript-like syntax but statically typed & sane

Inspirations: Java(Script), C#, Swift, Pony, ML, Haskell

### Semantics

* call-by-value
  (like Java, C, JS, ML; unlike Haskell, Nix)
* declarations are locally mutually recursive
* parametric, bounded polymorphism
* subtyping as subsumption, not coercion.
* no dynamic casts
* no inheritance

### Implementation(s)

* implemented in OCaml (leverages `wasm` libary)
* simple reference interpreter
* less simple compiler to WebAssembly
  * multipass with typed IR in each pass.
  * uniform representation, unboxed arithmetic
  * two-space gc (for now), gc between messages
* polymorphism by erasure

# The language

## Expressions

* Identifiers:  
  `x`, `foo_bar`, `test'`, `List`, `Map`

* Parentheses for grouping

* Type annotations (to help type inference):  
  `(42 : Int)`

## Blocks and declarations

```
  type Delta = Nat;
  func print() {
    Debug.print(Int.toText(counter));
  };
  let d : Delta = 42;
  var counter = 1;
  counter := counter + tmp;
  print();
```

* Semicolon after each declaration!
* Mutually recursive
* Mutable variables marked explicitly

## Control flow

- `if b { … }`
- `if b { … } else { … }`
- `switch x { case pat1 { e1 } … case _ { en } }`
- `while b { … }`
- `loop { … }`
- `loop { … } while b`
- `for pat in e { … }`

<!--
### Labels, break and continue

  - `label l exp`
  - `break l` (more generally, `break l exp`)
  - `continue l`

labels ensure control flow is structured (no gotos)
-->


# Primitive types

## Unbounded integers

`Int`

Inferred by default for negative literals.

Literals: `13`, `0xf4`, `-20`, `+1`, `1_000_000`


## Unbounded naturals

`Nat`

Non-negative, trap upon underflow.

Inferred by default for non-negative literals

Literals: `13`, `0xf4`, `1_000_000`

## Bounded numbers (trapping)

`Nat8`, `Nat16`, `Nat32`, `Nat64`,
`Int8`, `Int16`, `Int32`, `Int64`

Trap on over- and underflow. Wrap-on-trap and bit-manipulating operations available.

Needs type annotations (somewhere)

Literals: `13`, `0xf4`, `-20`, `1_000_000`

## Floating point numbers

`Float`

IEEE 754 double precision (64 bit) semantics, normalized NaN

Inferred for fractional literals

Literals: 0, -10, `2.71`, `-0.3e+15`, `3.141_592_653_589_793_12`

## Numeric operations

No surprises here

`- x`  
`a + b`  
`a & b`  
…

## Characters and text

`Char`, `Text`

Unicode! No random access.

 * `'x'`, `'\u{6a}'`, `'☃'`,
 * `"boo"`, `"foo \u{62}ar ☃"`
 * `"Concat" # "enation"`

## Booleans

`Bool`

Literals: `true`, `false`

`a or b`  
`a and b`  
`not b`  
`if b { e1 } else { e2 }`

# Functions

## Function types

* Simple functions:
  ```
  Int.toText : Int -> Text
  ```

* multiple arguments and return values  
  ```
  divRem : (Int, Int) -> (Int, Int)
  ```

* can be generic/polymorphic  
  ```
  Option.unwrapOr : <T>(?T, default : T) -> T
  ```

* first-class (can be passed around, stored)  
  ```
  map : <A, B>(f : A -> B, xs : [A]) -> [B]
  let funcs : [<T>(T) -> T] = …
  ```

## Function Declarations & Use

```
func add(x : Int, y : Int) : Int = x + y;

func applyNTimes<T>(n : Nat, x : T, f : T -> ()) {
  if n == 0 { return };
  f(x);
  applyNTimes(n-1, x, f);
}

applyNTimes<Text>(10, "Hello!", func(x) = { Debug.print(x) } );
```

* `func() { … }` short for `func() : () = { … }`
* Parametric functions
* Type instantiations may sometimes be omitted
* Anonymous functions (a.k.a. lambdas)


# Composite types

## Tuples

`(Bool, Float, Text)`

immutable, heterogeneous, fixed size

```
let tuple = (true, 1.2, "foo");
tuple.1 > 0.0;
let (_,_,t) = tuple;
```

## Options

`?Text`

is either a value of that type, or `null`

```
func foo(x : ?Text) : Text {
  switch x {
    case null { "No value" }
    case ?y { "Value: " # y }
  };
};
foo(null);
foo(?"Test");
```

## Arrays (immutable)

`[Text]`

```
let days = ["Monday", "Tuesday", … ];
assert(days.len() == 7);
assert(days[1] == "Tuesday");
// days[7] will trap (fixed size)
for d in days.values() { Debug.print(d) };
```

## Arrays (mutable)

`[var Nat]`

```
let counters = [var 1, 2, 3];
assert(counters.len() == 3);
counters[1] := counters[1] + 1;
// counters[3] will trap (fixed size)
```

## Records

`{name : Text; points : var Int}`

```
let player = { name = "Joachim";  var points = 0 };
Debug.print(
  player.name # " has " #
  Int.toText(player.points) # " points."
);
player.points += 1;
```

## Objects

`{ get : () -> Int; add : Int -> () }`

```
object self {
  var points = 0; // private by default
  public func get() = points;
  public func add(p : Int) { points += p };
}
```

Different syntax, same type as records

## Variants

`{ #invincible; #alive : Int; #dead }`

```
type Health = { #invincible; #alive : Nat; #dead };
func takeDamage(h : Health, p : Nat) : Health {
  switch h {
    case #invincible { #invincible }
    case #alive(hp) {
      if hp > p { #alive (hp-p) } else { #dead }
    }
    case #dead { #dead }
  }
}
```

# Packages and modules

## Modules

```
// the type of base/Int.mo
module {
  toText : Int -> Text;
  abs : Int -> Nat;
  …
}
```

types and values like objects

but restricted to _static_ content (pure, no state, …)

## Module imports

```
import Debug "mo:base/Debug";
import Int "mo:base/Int";
```

`base` package provides basic features.

More libraries popping up!

# Platform features

## Actor types

Like object types, but marked as `actor`:

```
type Receiver = actor { recv : Text -> async Nat };
type Broadcast = actor {
  register : Receiver -> ();
  send : Text -> async Nat;
}
```

_sharable_ arguments and _no_ or _async_ result type.

“canister” ≈ “actor”

## sharable ≈ serializable

  - all primitive types
  - records, tuples, arrays, variants, options  
    with immutable sharable components
  - `actor` types
  - `shared` function type<br/><br/>**Not sharable:**
  - mutable things
  - local functions
  - objects (with methods)


## A complete actor

```
import Array "mo:base/Array";
actor {
  var r : [Receiver] = [];
  public func register(a : Receiver) {
    r := Array.append(r, [a]);
  };
  public func send(t : Text) : async Nat {
    var sum := 0;
    for a in r.values() {
      sum += await a.recv(t);
    };
    return sum;
  };
}
```

a typical canister main file

## Async/await

`async T`

asychronous future or promise

introduced by `async { … }`  
(implicit in async function declaration)

`await e`  
suspends computation pending `e`'s result

## Actor import

```
import Broadcast "ic:ABCDEF23";
actor Self {
  public func go() {
    Broadcast.register(Self);
  };
  public func recv(msg : Text) : async Nat {
    …
  }
}
```

## Principal and caller

```
actor Self {
  let myself : Principal = Principal.fromActor(Self);
  public shared(context) func hello() : async Text {
    if context.caller == myself {
      "Talking to yourself is the first sign of madness";
    } else {
      "Hello, nice to see you";
    };
  };
}
```

`Principal`: identity of a user or canister/actor

# Type system

## Structural

```
type Health1 = { #invincible; #alive : Nat; #dead };
type Health2 = { #invincible; #alive : Nat; #dead };

let takeDamage : (Health1, Nat) -> Health1 = …;
let h : Health2 = #invincible;
let h' = takeDamage(h, 100); // works
```

Type definitions  
do not create types,  
but name existing types

## Subtyping

`Mortal <: Health`

```
type Health = { #invincible; #alive : Nat; #dead };
type Mortal = { #alive : Nat; #dead };

let takeDamage : (Health, Nat) -> Health = …;
let h : Mortal = #alive 1000;
let h' = takeDamage(h, 100); // also works
```

`t1 <: t2`: `t1` can be used wherever `t2` is expected


## Generic types

```
type List<T> = ?{head : T; tail : List<T>};

…
let l : List<Nat> = ?{head = 0; tail = ?{head = 1 ; tail = null }};
```

# Fin

## Not covered

 * Polymorphic functions with type bounds
 * Classes
 * Error handling (`try … catch …` & `throw …`)

# Old slides

### Classes

Classes as functions returning objects:
```
 class Counter(init : Int) {
    private var state : Int = init;
    public func inc() { state += 1; };
    public func get() : Int { state; };
  }
```

Class instantiation as function call (no `new`):
```
let c = Counter(666);
c.inc();
let 667 = c.get();
```


### Generic Classes

```
class Dict< K, V > (cmp : (K,K)-> Int ) {
  add(k: K, v: V) { ... };
  find(k: K) : ? V { ... };
};
```

```
let d = Dict<Int,Text> (func (i:Int, j:Int) : Int = i - j);
d.add(1,"Alice");
let ? name = d.find(1);
```

### Language prelude

* connects internal primitives with surface syntax (types, operations)
* conversions like `intToNat32`
* side-effecting operations `debugPrintInt`
  (tie into execution environment)
* utilities like `hashInt`, `clzNat32`


# Sample App


### Implementing *Chat*

* type example
* one server actor
* multiple clients, each an instance of (actor) class Client.

### Chat Server

```
actor Server {
  private var clients : List<Client> = null;

  private shared broadcast(message : Text) {
    var next = clients;
    loop {
      switch next {
        case null { return; }
        case ?l { l.head.send(message); next := l.tail }
      };
    };
  };
```
```
  public func subscribe(client : Client) : async Post {
    let cs = {head = client; var tail = clients};
    clients := ?cs;
    return broadcast;
  };
};
```


### Example: The client class
<!--
 * should we remove the name and server fields? They aren't used, I believe, but somewhat illustrative.
* The fields would be  needed for unsubscribing etc, unless we return an unsubscribe capability...
 * Also, subscribe could just take send, not a whole client.

-->
```
type Server = actor { subscribe : Client -> async Post; };

actor class Client() = this {
  private var name : Text = "";
  public func start(n : Text , s : Server) {
    name := n;
    let _ = async {
       let post = await s.subscribe(this);
       post("hello from " # name);
       post("goodbye from " # name);
    }
  };
```
```
  public func send(msg : Text) {
    debugPrint(name # " received " # msg # "\n");
  };
};
```
### Example: test

test

```
let bob = Client();
let alice = Client();
let charlie = Client();

bob.start("Bob", Server);
alice.start("Alice", Server);
charlie.start("Charlie", Server);
```
output

```
[nix-shell:~/motoko/guide]$ ../src/moc -r chat.mo
charlie received hello from bob
alice received hello from bob
bob received hello from bob
charlie received goodbye from bob
alice received goodbye from bob
bob received goodbye from bob
charlie received hello from alice
alice received hello from alice
bob received hello from alice
charlie received goodbye from alice
alice received goodbye from alice
bob received goodbye from alice
charlie received hello from charlie
alice received hello from charlie
bob received hello from charlie
charlie received goodbye from charlie
alice received goodbye from charlie
bob received goodbye from charlie
```
