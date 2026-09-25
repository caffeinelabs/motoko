//MOC-FLAG -W M0223
func check<T>(t1 : T, t2 : T) : [T] = [t1, t2]; // used to check type equality
func check3<T>(t1 : T, t2 : T, t3 : T) : [T] = [t1, t2, t3]; // used to check type equality

type Order = { #less; #equal; #greater };

// Mock functions for Array module
module Array {
  public func tabulate<T>(_size : Nat, _f : Nat -> T) : [T] = [];
  public func find<T>(_array : [T], _predicate : T -> Bool) : ?T = null;
  public func findIndex<T>(_array : [T], _predicate : T -> Bool) : ?Nat = null;
  public func forEach<T>(_array : [T], _f : T -> ()) {};
  public func map<T, R>(_array : [T], _f : T -> R) : [R] = [];
  public func mapToMutable<T, R>(_array : [T], _f : T -> R) : [var R] = [var];
  public func filter<T>(_array : [T], _f : T -> Bool) : [T] = [];
  public func mapEntries<T, R>(_array : [T], _f : (T, Nat) -> R) : [R] = [];
  public func flatMap<T, R>(_array : [T], _f : T -> [R]) : [R] = [];
  public func foldRight<T, A>(_array : [T], _base : A, _f : (T, A) -> A) : A = _base;
  public func all<T>(_array : [T], _predicate : T -> Bool) : Bool = true;
  public func any<T>(_array : [T], _predicate : T -> Bool) : Bool = false;
  public func sort<T>(_array : [T], _compare : (T, T) -> Order) : [T] = [];
};

// Mock functions for VarArray module
module VarArray {
  public func tabulate<T>(_size : Nat, _f : Nat -> T) : [var T] = [var];
  public func find<T>(_array : [var T], _predicate : T -> Bool) : ?T = null;
  public func findIndex<T>(_array : [var T], _predicate : T -> Bool) : ?Nat = null;
  public func forEach<T>(_array : [var T], _f : T -> ()) {};
  public func map<T, R>(_array : [var T], _f : T -> R) : [var R] = [var];
  public func mapToImmutable<T, R>(_array : [var T], _f : T -> R) : [R] = [];
  public func mapInPlace<T>(_array : [var T], _f : T -> T) {};
  public func filter<T>(_array : [var T], _f : T -> Bool) : [var T] = [var];
  public func mapEntries<T, R>(_array : [var T], _f : (T, Nat) -> R) : [var R] = [var];
  public func flatMap<T, R>(_array : [var T], _f : T -> [R]) : [var R] = [var];
  public func foldRight<T, A>(_array : [var T], _base : A, _f : (T, A) -> A) : A = _base;
  public func all<T>(_array : [var T], _predicate : T -> Bool) : Bool = true;
  public func any<T>(_array : [var T], _predicate : T -> Bool) : Bool = false;
};

// Mock functions for Iter module
type Iter<T> = { next : () -> ?T };
module Iter {
  public func empty<T>() : Iter<T> {
    object {
      public func next() : ?T {
        null;
      };
    };
  };
  public func forEach<T>(_iter : Iter<T>, _f : T -> ()) {};
  public func map<T, R>(_iter : Iter<T>, _f : T -> R) : Iter<R> = empty();
  public func filter<T>(_iter : Iter<T>, _f : T -> Bool) : Iter<T> = _iter;
  public func filterMap<T, R>(_iter : Iter<T>, _f : T -> ?R) : Iter<R> = empty();
  public func flatMap<T, R>(_iter : Iter<T>, _f : T -> Iter<R>) : Iter<R> = empty();
  public func takeWhile<T>(_iter : Iter<T>, _predicate : T -> Bool) : Iter<T> = _iter;
  public func dropWhile<T>(_iter : Iter<T>, _predicate : T -> Bool) : Iter<T> = _iter;
  public func zipWith<T, U, R>(_iter1 : Iter<T>, _iter2 : Iter<U>, _f : (T, U) -> R) : Iter<R> = empty();
  public func zipWith3<T, U, V, R>(_iter1 : Iter<T>, _iter2 : Iter<U>, _iter3 : Iter<V>, _f : (T, U, V) -> R) : Iter<R> = empty();
  public func all<T>(_iter : Iter<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_iter : Iter<T>, _predicate : T -> Bool) : Bool = false;
  public func find<T>(_iter : Iter<T>, _predicate : T -> Bool) : ?T = null;
  public func findIndex<T>(_iter : Iter<T>, _predicate : T -> Bool) : ?Nat = null;
  public func foldLeft<T, A>(_iter : Iter<T>, _base : A, _f : (A, T) -> A) : A = _base;
  public func foldRight<T, A>(_iter : Iter<T>, _base : A, _f : (T, A) -> A) : A = _base;
  public func unfold<T, S>(_init : S, _f : S -> ?(T, S)) : Iter<T> = empty();
};

// Mock functions for List module
type List<T> = {
  var blocks : [var [var ?T]];
  var blockIndex : Nat;
  var elementIndex : Nat;
};
module List {
  public func empty<T>() : List<T> = {
    var blocks = [var];
    var blockIndex = 0;
    var elementIndex = 0;
  };
  public func filter<T>(_list : List<T>, _predicate : T -> Bool) : List<T> = _list;
  public func map<T, R>(_list : List<T>, _f : T -> R) : List<R> = empty();
  public func filterMap<T, R>(_list : List<T>, _f : T -> ?R) : List<R> = empty();
  public func find<T>(_list : List<T>, _predicate : T -> Bool) : ?T = null;
  public func findIndex<T>(_list : List<T>, _predicate : T -> Bool) : ?Nat = null;
  public func findLastIndex<T>(_list : List<T>, _predicate : T -> Bool) : ?Nat = null;
  public func all<T>(_list : List<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_list : List<T>, _predicate : T -> Bool) : Bool = false;
};

// Mock functions for pure List module
module PureList {
  public func empty<T>() : PureList<T> = null;
  public func forEach<T>(_list : PureList<T>, _f : T -> ()) {};
  public func map<T, R>(_list : PureList<T>, _f : T -> R) : PureList<R> = empty();
  public func filter<T>(_list : PureList<T>, _predicate : T -> Bool) : PureList<T> = empty();
  public func partition<T>(_list : PureList<T>, _predicate : T -> Bool) : (PureList<T>, PureList<T>) = (empty(), empty());
  public func find<T>(_list : PureList<T>, _predicate : T -> Bool) : ?T = null;
  public func findIndex<T>(_list : PureList<T>, _predicate : T -> Bool) : ?Nat = null;
  public func all<T>(_list : PureList<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_list : PureList<T>, _predicate : T -> Bool) : Bool = false;
  public func tabulate<T>(_size : Nat, _f : Nat -> T) : PureList<T> = empty();
};

// Mock functions for Queue module
type Queue<T> = {
  var front : ?Node<T>;
  var back : ?Node<T>;
  var size : Nat;
};

type Node<T> = {
  value : T;
  var next : ?Node<T>;
  var previous : ?Node<T>;
};
module Queue {
  public func empty<T>() : Queue<T> = {
    var front = null;
    var back = null;
    var size = 0;
  };
  public func all<T>(_queue : Queue<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_queue : Queue<T>, _predicate : T -> Bool) : Bool = false;
  public func forEach<T>(_queue : Queue<T>, _f : T -> ()) {};
  public func map<T, R>(_queue : Queue<T>, _f : T -> R) : Queue<R> = empty();
  public func filter<T>(_queue : Queue<T>, _predicate : T -> Bool) : Queue<T> = _queue;
};

// Mock functions for pure Queue module
type PureList<T> = ?(T, PureList<T>);
type PureQueue<T> = (PureList<T>, Nat, PureList<T>);
module PureQueue {
  public func empty<T>() : PureQueue<T> = (null, 0, null);
  public func all<T>(_queue : PureQueue<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_queue : PureQueue<T>, _predicate : T -> Bool) : Bool = false;
  public func forEach<T>(_queue : PureQueue<T>, _f : T -> ()) {};
  public func filter<T>(_queue : PureQueue<T>, _predicate : T -> Bool) : PureQueue<T> = _queue;
  public func map<T, R>(_queue : PureQueue<T>, _f : T -> R) : PureQueue<R> = empty();
  public func filterMap<T, R>(_queue : PureQueue<T>, _f : T -> ?R) : PureQueue<R> = empty();
};

// Mock functions for Stack module
type Stack<T> = {
  var top : PureList<T>;
  var size : Nat;
};
module Stack {
  public func empty<T>() : Stack<T> = {
    var top = null;
    var size = 0;
  };
  public func tabulate<T>(_size : Nat, _f : Nat -> T) : Stack<T> = empty();
  public func all<T>(_stack : Stack<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_stack : Stack<T>, _predicate : T -> Bool) : Bool = false;
  public func forEach<T>(_stack : Stack<T>, _f : T -> ()) {};
  public func map<T, R>(_stack : Stack<T>, _f : T -> R) : Stack<R> = empty();
  public func filter<T>(_stack : Stack<T>, _predicate : T -> Bool) : Stack<T> = _stack;
  public func filterMap<T, R>(_stack : Stack<T>, _f : T -> ?R) : Stack<R> = empty();
};

// Mock functions for Set module
type Set<T> = Set.Set<T>;
type Compare<T> = (T, T) -> { #less; #equal; #greater };
module Set {
  public type Node<T> = {
    #leaf : Leaf<T>;
    #internal : Internal<T>;
  };

  public type Data<T> = {
    elements : [var ?T];
    var count : Nat;
  };

  public type Internal<T> = {
    data : Data<T>;
    children : [var ?Node<T>];
  };

  public type Leaf<T> = {
    data : Data<T>;
  };

  public type Set<T> = {
    var root : Node<T>;
    var size : Nat;
  };
  public func empty<T>() : Set<T> = {
    var root = #leaf { data = { elements = [var]; var count = 0 } };
    var size = 0;
  };
  public func retainAll<T>(_set : Set<T>, _compare : Compare<T>, _predicate : T -> Bool) : Bool = true;
  public func forEach<T>(_set : Set<T>, _f : T -> ()) {};
  public func filter<T>(_set : Set<T>, _compare : Compare<T>, _predicate : T -> Bool) : Set<T> = _set;
  public func map<T, R>(_set : Set<T>, _f : T -> R) : Set<R> = empty();
  public func filterMap<T, R>(_set : Set<T>, _compare : Compare<R>, _f : T -> ?R) : Set<R> = empty();
  public func all<T>(_set : Set<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_set : Set<T>, _predicate : T -> Bool) : Bool = false;
};

// Mock functions for pure Set module
type PureSet<T> = PureSet.Set<T>;
module PureSet {
  public type Tree<T> = {
    #red : (Tree<T>, T, Tree<T>);
    #black : (Tree<T>, T, Tree<T>);
    #leaf;
  };

  public type Set<T> = { size : Nat; root : Tree<T> };

  public func empty<T>() : PureSet<T> = { size = 0; root = #leaf };
  public func forEach<T>(_set : PureSet<T>, _f : T -> ()) {};
  public func filter<T>(_set : PureSet<T>, _compare : Compare<T>, _predicate : T -> Bool) : PureSet<T> = _set;
  public func map<T, R>(_set : PureSet<T>, _f : T -> R) : PureSet<R> = empty();
  public func filterMap<T, R>(_set : PureSet<T>, _compare : Compare<R>, _f : T -> ?R) : PureSet<R> = empty();
  public func all<T>(_set : PureSet<T>, _predicate : T -> Bool) : Bool = true;
  public func any<T>(_set : PureSet<T>, _predicate : T -> Bool) : Bool = false;
};

// Mock functions for Map module
type Map<K, V> = Map.Map<K, V>;
module Map {
  public type Node<K, V> = {
    #leaf : Leaf<K, V>;
    #internal : Internal<K, V>;
  };

  public type Data<K, V> = {
    kvs : [var ?(K, V)];
    var count : Nat;
  };

  public type Internal<K, V> = {
    data : Data<K, V>;
    children : [var ?Node<K, V>];
  };

  public type Leaf<K, V> = {
    // why the extra indirection?
    data : Data<K, V>;
  };

  public type Map<K, V> = {
    var root : Node<K, V>;
    var size : Nat;
  };
  public func empty<K, V>() : Map<K, V> = {
    var root = #leaf { data = { kvs = [var]; var count = 0 } };
    var size = 0;
  };
  public func map<K, V, R>(_map : Map<K, V>, _f : (K, V) -> R) : Map<K, R> = empty();
  public func forEach<K, V>(_map : Map<K, V>, _f : (K, V) -> ()) {};
  public func filter<K, V>(_map : Map<K, V>, _compare : Compare<K>, _predicate : (K, V) -> Bool) : Map<K, V> = _map;
  public func all<K, V>(_map : Map<K, V>, _predicate : (K, V) -> Bool) : Bool = true;
  public func any<K, V>(_map : Map<K, V>, _predicate : (K, V) -> Bool) : Bool = false;
  public func toText<K, V>(_map : Map<K, V>, _keyToText : K -> Text, _valueToText : V -> Text) : Text = "";
  public func fromIter<K, V>(_iter : Iter<(K, V)>, _compare : (K, K) -> Order) : Map<K, V> = empty();
};

// Mock functions for pure Map module
type PureMap<K, V> = PureMap.Map<K, V>;
module PureMap {
  public type Map<K, V> = {
    size : Nat;
    root : Tree<K, V>;
  };
  public type Tree<K, V> = {
    #red : (Tree<K, V>, K, V, Tree<K, V>);
    #black : (Tree<K, V>, K, V, Tree<K, V>);
    #leaf;
  };
  public func empty<K, V>() : PureMap<K, V> = { size = 0; root = #leaf };
  public func all<K, V>(_map : PureMap<K, V>, _predicate : (K, V) -> Bool) : Bool = true;
  public func any<K, V>(_map : PureMap<K, V>, _predicate : (K, V) -> Bool) : Bool = false;
  public func forEach<K, V>(_map : PureMap<K, V>, _f : (K, V) -> ()) {};
  public func filter<K, V>(_map : PureMap<K, V>, _compare : Compare<K>, _predicate : (K, V) -> Bool) : PureMap<K, V> = _map;
  public func map<K, V, R>(_map : PureMap<K, V>, _f : (K, V) -> R) : PureMap<K, R> = empty();
  public func toText<K, V>(_map : PureMap<K, V>, _keyToText : K -> Text, _valueToText : V -> Text) : Text = "";
};

// Mock functions for Result module
type Result<T, E> = { #ok : T; #err : E };
module Result {
  public func mapOk<T, R, E>(result : Result<T, E>, f : T -> R) : Result<R, E> = switch result {
    case #err(e) { #err(e) }
    case #ok(r) { #ok(f(r)) }
  };
  public func mapErr<T, E, F>(result : Result<T, E>, f : E -> F) : Result<T, F> = switch result {
    case #ok(r) { #ok(r) }
    case #err(e) { #err(f(e)) }
  };
  public func forOk<T, E>(_result : Result<T, E>, _f : T -> ()) {};
  public func forErr<T, E>(_result : Result<T, E>, _f : E -> ()) {};
};

// Mock functions for Option module
module Option {
  public func map<T, R>(_option : ?T, _f : T -> R) : ?R = null;
};

// Mock functions for Tuple modules
module Tuple2 {
  public func makeToText<A, B>(_f1 : A -> Text, _f2 : B -> Text) : (A, B) -> Text = func(_a, _b) = "";
};

module Tuple3 {
  public func makeToText<A, B, C>(_f1 : A -> Text, _f2 : B -> Text, _f3 : C -> Text) : (A, B, C) -> Text = func(_a, _b, _c) = "";
};

// Mock comparison and conversion functions
module Int {
  public func compare(_a : Int, _b : Int) : { #less; #equal; #greater } = #equal;
};
func natCompare(_a : Nat, _b : Nat) : { #less; #equal; #greater } = #equal;
func textCompare(_a : Text, _b : Text) : { #less; #equal; #greater } = #equal;
func natToText(_n : Nat) : Text = "";

let ar = [1, 2, 3];
let varAr : [var Nat] = [var 1, 2, 3];
let varArInt : [var Int] = [var 1, 2, 3];
let iter : Iter<Nat> = { next = func() : ?Nat = null };
let iterText : Iter<Text> = { next = func() : ?Text = null };
let iterChar : Iter<Char> = { next = func() : ?Char = null };
let list : List<Nat> = List.empty();
let pureList : PureList<Nat> = null;
let queue : Queue<Nat> = Queue.empty();
let pureQueue : PureQueue<Nat> = PureQueue.empty();
let stack : Stack<Nat> = Stack.empty();
let set : Set<Nat> = Set.empty();
let pureSet : PureSet<Nat> = PureSet.empty();
let mapInstance : Map<Nat, Text> = Map.empty();
let pureMap : PureMap<Nat, Text> = PureMap.empty();
let resultOk : Result<Nat, Text> = #ok(1);
let resultErr : Result<Nat, Text> = #err("");
let optionSome : ?Nat = ?1;

// Array module explicit type instantiation tests
let _ = Array.tabulate(4, func i = i * 2);
let _ = Array.find(ar, func x = x > 8);
let _ = Array.findIndex(['A', 'B', 'C'], func x = x == 'C');
Array.forEach(ar, func _ {});
let _ = Array.map(ar, func x = x * 2);
let _ = Array.mapToMutable<Nat, Int>(ar, func x = x * 2);
let _ = Array.filter(ar, func x = x % 2 == 0);
let _ = Array.mapEntries(ar, func(x, i) = i * x);
let _ = Array.flatMap(ar, func x = [x, -x]);
let _ = Array.foldRight(ar, "", func(x, acc) = natToText(x) # acc);
let _ = Array.all(ar, func x = x > 0);
let _ = Array.any(ar, func x = x > 3);
let _ = Array.sort(ar, func(a, b) { Int.compare(b, a) });

// VarArray module explicit type instantiation tests
let va1 = VarArray.tabulate<Int>(4, func i = i * 2);
let va2 : [var Int] = VarArray.tabulate(4, func i = i * 2);
let _ = check(va1, va2);
let _ = VarArray.find(varAr, func x = x > 8);
let _ = VarArray.findIndex([var 'A', 'B', 'C'], func x = x == 'C');
VarArray.forEach(varAr, func(x) {});
let va3 = VarArray.map<Nat, Int>(varAr, func x = x * 2);
let va4 : [var Int] = VarArray.map(varAr, func x = x * 2);
let _ = check(va3, va4);
let _ = VarArray.map(varArInt, func x = x * 2);
let _ = VarArray.mapToImmutable(varAr, func x = x * 2);
VarArray.mapInPlace(varAr, func x = x * 3);
let _ = VarArray.filter(varAr, func x = x % 2 == 0);
let va5 = VarArray.mapEntries<Nat, Int>(varAr, func(x, i) = i * x);
let va6 : [var Int] = VarArray.mapEntries(varAr, func(x, i) = i * x);
let _ = check(va5, va6);
let va7 = VarArray.flatMap<Nat, Int>(varAr, func x = [x, -x]);
let va8 : [var Int] = VarArray.flatMap(varAr, func x = [x, -x]);
let _ = check(va7, va8);
let _ = VarArray.foldRight(varAr, "", func(x, acc) = natToText(x) # acc);
let _ = VarArray.all(varAr, func x = x > 0);
let _ = VarArray.any(varAr, func x = x > 3);

// Iter module explicit type instantiation tests
Iter.forEach(iter, func _ {});
let _ = Iter.map(iter, func x = x * 2);
let _ = Iter.filter(iter, func x = x % 2 == 0);
let _ = Iter.filterMap(iter, func x = if x % 2 == 0 { ?x } else { null });
let _ = Iter.flatMap(iter, func x = iter);
let _ = Iter.takeWhile(iter, func x = x < 4);
let _ = Iter.dropWhile(iter, func x = x < 4);
let _ = Iter.zipWith(iterText, iterText, func(a, b) = a # b);
let _ = Iter.zipWith3(iterText, iterText, iterText, func(a, b, c) = a # b # c);
let _ = Iter.all(iter, func x = x < 4);
let _ = Iter.any(iter, func x = x == 2);
let _ = Iter.find(iter, func x = x % 2 == 0);
let _ = Iter.findIndex(iterChar, func x = x == 'C');
let _ = Iter.foldLeft(iterText, "S", func(acc, x) = "(" # acc # x # ")");
let _ = Iter.foldRight(iterText, "S", func(x, acc) = "(" # x # acc # ")");
let _ = Iter.unfold(1, func x = if x <= 3 { ?(x, x + 1) } else { null });

// List module explicit type instantiation tests
let _ = List.filter(list, func x = x % 2 == 0);
let l1 = List.map<Nat, Int>(list, func x = x * 2);
let l2 : List<Int> = List.map(list, func x = x * 2);
let _ = check(l1, l2);
let l3 = List.filterMap<Nat, Int>(list, func x = if x % 2 == 0 { ?(x * 2) } else { null });
let l4 : List<Int> = List.filterMap(list, func x = if x % 2 == 0 { ?(x * 2) } else { null });
let _ = check(l3, l4);
let _ = List.find(list, func x = x > 8);
let _ = List.findIndex(list, func i = i % 2 == 0);
let _ = List.findLastIndex(list, func i = i % 2 == 0);
let _ = List.all(list, func x = x > 1);

// pure List module explicit type instantiation tests
PureList.forEach(pureList, func n = ());
let _ = PureList.filter(pureList, func n = n != 1);
let _ = PureList.map(pureList, func n = n * 2);
let _ = PureList.partition(pureList, func n = n != 1);
let _ = PureList.find(pureList, func n = n > 1);
let _ = PureList.findIndex(pureList, func n = n > 1);
let _ = PureList.all(pureList, func n = n > 1);
let _ = PureList.any(pureList, func n = n > 1);
let _ = PureList.tabulate(3, func n = n * 2);

// Queue module explicit type instantiation tests
let _ = Queue.all(queue, func x = x % 2 == 0);
let _ = Queue.any(queue, func x = x > 2);
Queue.forEach(queue, func _ {});
let q1 = Queue.map<Nat, Int>(queue, func x = x * 2);
let q2 : Queue<Int> = Queue.map(queue, func x = x * 2);
let _ = check(q1, q2);
let _ = Queue.filter(queue, func x = x % 2 == 0);

// pure Queue module explicit type instantiation tests
let _ = PureQueue.all(pureQueue, func n = n > 1);
let _ = PureQueue.any(pureQueue, func n = n > 1);
PureQueue.forEach(pureQueue, func _ {});
let _ = PureQueue.filter(pureQueue, func n = n != 1);
let _ = PureQueue.map(pureQueue, func n = n * 2);
let _ = PureQueue.filterMap(pureQueue, func n = if n % 2 == 0 { ?n } else { null });

// Stack module explicit type instantiation tests
let st1 = Stack.tabulate<Int>(3, func i = 2 * i);
let st2 : Stack<Int> = Stack.tabulate(3, func i = 2 * i);
let _ = check(st1, st2);
let _ = Stack.all(stack, func n = n % 2 == 0);
let _ = Stack.any(stack, func n = n == 2);
Stack.forEach(stack, func _ {});
let st3 = Stack.map<Nat, Int>(stack, func n = 2 * n);
let st4 : Stack<Int> = Stack.map(stack, func n = 2 * n);
let _ = check(st3, st4);
let _ = Stack.filter(stack, func n = n % 2 == 0);
let st5 = Stack.filterMap<Nat, Int>(stack, func n = if n % 2 == 0 { ?n } else { null });
let st6 : Stack<Int> = Stack.filterMap(stack, func n = if n % 2 == 0 { ?n } else { null });
let _ = check(st5, st6);

// Set module explicit type instantiation tests
let _ = Set.retainAll(set, natCompare, func n = n % 2 == 0);
Set.forEach(set, func _ {});
let _ = Set.filter(set, natCompare, func n = n % 2 == 0);
let s1 = Set.map<Nat, Text>(set, func n = natToText(n));
let s1i = Set.map(set, func n = natToText(n));
let s2 : Set<Text> = Set.map(set, func n = natToText(n));
let _ = check3(s1, s1i, s2);
let s3 = Set.filterMap<Nat, Text>(
  set,
  textCompare,
  func n = if n % 2 == 0 { ?natToText(n) } else { null },
);
let s3i = Set.filterMap(
  set,
  textCompare,
  func n = if n % 2 == 0 { ?natToText(n) } else { null },
);
let s4 : Set<Text> = Set.filterMap(
  set,
  textCompare,
  func n = if n % 2 == 0 { ?natToText(n) } else { null },
);
let _ = check3(s3, s3i, s4);
let _ = Set.all(set, func n = n < 10);
let _ = Set.any(set, func n = n > 5);

// pure Set module explicit type instantiation tests
PureSet.forEach(pureSet, func _ {});
let _ = PureSet.filter(pureSet, natCompare, func n = n % 2 == 0);
let ps1 = PureSet.map<Nat, Text>(pureSet, func n = natToText(n));
let ps2 = PureSet.map(pureSet, func n = natToText(n));
let _ = check(ps1, ps2);
let ps3 = PureSet.filterMap<Nat, Text>(
  pureSet,
  textCompare,
  func n = if n % 2 == 0 { ?natToText(n) } else { null },
);
let ps4 = PureSet.filterMap(
  pureSet,
  textCompare,
  func n = if n % 2 == 0 { ?natToText(n) } else { null },
);
let _ = check(ps3, ps4);
let _ = PureSet.all(pureSet, func n = n < 10);
let _ = PureSet.any(pureSet, func n = n > 5);

// Map module explicit type instantiation tests
Map.forEach(mapInstance, func(key, value) {});
let _ = Map.filter(mapInstance, natCompare, func(key, value) = key % 2 == 0);
let m1 = Map.map<Nat, Text, Text>(mapInstance, func(key, value) = natToText(key));
let m1i = Map.map(mapInstance, func(key, value) = natToText(key));
let m2 : Map<Nat, Text> = Map.map(mapInstance, func(key, value) = natToText(key));
let _ = check3(m1, m1i, m2);
let _ = Map.all(mapInstance, func(k, v) = v == natToText(k));
let _ = Map.any(mapInstance, func(k, v) = k >= 0);
let _ = Map.toText(mapInstance, natToText, func t = t);
let _ = Map.fromIter([(0, "0")].values(), natCompare);

// pure Map module explicit type instantiation tests
let _ = PureMap.all(pureMap, func(k, v) = v == natToText(k));
let _ = PureMap.any(pureMap, func(k, v) = k >= 0);
PureMap.forEach(pureMap, func(key, value) {});
let pm1 = PureMap.map<Nat, Text, Text>(pureMap, func(key, value) = natToText(key));
let pm2 = PureMap.map(pureMap, func(key, value) = natToText(key));
let _ = check(pm1, pm2);
let _ = PureMap.filter(
  pureMap,
  natCompare,
  func(key, value) {
    key % 2 == 0;
  },
);
let _ = PureMap.toText(pureMap, natToText, func t = t);

// Result module explicit type instantiation tests
let _ = Result.mapOk(resultOk, func x = x * 2);
let _ = Result.mapErr(resultErr, func x = x # "!");
Result.forOk(resultOk, func x {});
Result.forErr(resultErr, func x {});

// Option module explicit type instantiation tests
let _ = Option.map(optionSome, func x = x + 1);

// Tuple module explicit type instantiation tests
// Future work: support these. It needs special case for simple func bodies to create constraints without calling infer
// let _ = Tuple2.makeToText(natToText, func x = x);
// let _ = Tuple3.makeToText(natToText, func x = x, natToText);

module Issue5418 {
  public func test() : async [Int] {
    Array.sort(
      [-1, 0, 1],
      func(a, b) { Int.compare(b, a) },
    );
  };
};

module Return {
  public func test1() {
    let ar = Array.tabulate<Bool>(3, func i {
      if i == 0 { return false };
      true;
    });
    assert ar == [];
  };
  public func test2() {
    let ar = Array.tabulate(3, func i {
      if i == 0 { return false };
      true;
    });
    assert ar == [];
  };
};
Return.test1();
Return.test2();

//SKIP comp
//SKIP run-ir
//SKIP run-low
