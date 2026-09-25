object Random {
  let max = 0xF_FFFF_FFFF_FFFF_FFFF;
  let seed = 4711;
  var number = seed;

  public func nextInt() : Int {
    let number = +nextNat();
    let sign = if number % 2 == 0 { -1 } else { +1 };
    number * sign;
  };

  public func nextNat() : Nat {
    number := (123138118391 * number + 133489131) % 9_999_999;
    (number * number) % max;
  };
};

func serializeInt(a : [Int]) : Blob = to_candid (a);
func deserialzeInt(b : Blob) : ?[Int] = from_candid (b);

func serializeNat(a : [Nat]) : Blob = to_candid (a);
func deserialzeNat(b : Blob) : ?[Nat] = from_candid (b);

let rounds = 100000;

var count = 0;
while count < rounds {
  let number = Random.nextInt();
  assert ((?[number]) == deserialzeInt(serializeInt([number])));
  count += 1;
};

count := 0;
while count < rounds {
  let number = Random.nextNat();
  assert ((?[number]) == deserialzeNat(serializeNat([number])));
  count += 1;
};

//SKIP run
//SKIP run-ir
//SKIP run-low
