// basic newtype declaration and usage

newtype Time = Int;

// construction
let time : Time = Time(123);

// unwrap
let n : Int = time.unwrap;
assert (n == 123);

// roundtrip
let time2 = Time(42);
assert (time2.unwrap == 42);

// newtype with negative value
let neg = Time(-1);
assert (neg.unwrap == -1);

// newtype in function signatures
func addTimes(a : Time, b : Time) : Int {
  a.unwrap + b.unwrap
};
assert (addTimes(Time(10), Time(20)) == 30);

// nested usage
func wrapAndUnwrap(x : Int) : Int {
  Time(x).unwrap
};
assert (wrapAndUnwrap(99) == 99);

// constructor as a first-class function
let ctor : Int -> Time = Time;
let t3 = ctor(7);
assert (t3.unwrap == 7);
