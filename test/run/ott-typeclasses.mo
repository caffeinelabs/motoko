// test realistic f-bound polymorphism using a contrived implementation of type classes
// NB: it's contrived, because we don't need the dictionary *type* parameters

type Eq<T> = {
  eq: (T, T) -> Bool
};

func equal<W <: Eq<T>, T>(w:W, t1 : T, t2 : T) : Bool = w.eq(t1, t2);

type Order = { #LT; #EQ; #GT; };

type Ord<T> = {
  eq : (T, T) -> Bool;
  cmp: (T, T) -> Order;
};

func compare<W <: Ord<T>, T>(w : W, t1 : T, t2 : T) : Order = w.cmp(t1, t2);

object ordInt {
  public func eq(t1 : Int, t2:Int) : Bool = t1 == t2;
  public func cmp(t1 : Int, t2 : Int) : Order =
    if t1 < t2 { #LT }
    else if t1 > t2 { #GT }
    else { #EQ };
};

type List<T> = ?(T, List<T>);

class OrdList<W <: Ord<T>, T>(w : W) = this {
  public func eq(ts1 : List<T>, ts2: List<T>) : Bool =
    switch cmp(ts1, ts2) {
      case #EQ { true }
      case _ { false }
    };

  public func cmp(ts1 : List<T>, ts2: List<T>) : Order =
    switch (ts1, ts2) {
      case (null, null) { #EQ }
      case (null, ? _) { #LT }
      case (?(t1, ts1), ?(t2, ts2)) {
        switch compare<W, T>(w, t1, t2) {
          case #EQ { (compare<OrdList<W, T>, List<T>> (this, ts1, ts2)) }
          case other { other }
        }
      }
      case (? _, null) { #GT }
    }
};

assert(equal<Ord<Int>, Int>(ordInt, 0, 0));

assert(not (equal<Ord<Int>, Int>(ordInt, -1, 0)));

type OLI = OrdList<Ord<Int>,Int>;
let oli = OrdList<Ord<Int>, Int>(ordInt);

switch compare<OLI, List<Int>>(oli, null, null) {
  case #EQ { () }
  case _ { (assert false) }
};

switch compare<OLI, List<Int>>(oli, ?(1, null), null) {
  case #GT { () }
  case _ { (assert false) }
};
