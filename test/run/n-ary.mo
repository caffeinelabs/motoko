//MOC-FLAG -A=M0194
func foo_0_0 () : () = ();

func foo_0_1 () : (Int) = 1;

func foo_0_2 () : (Int, Int) = (1,2);

func foo_0_2_block () : (Int, Int) { let x = 1; let y = 2; (x,y) };

func foo_0_return_2 () : (Int, Int) { return (1,2); (3,4) };

func foo_0_break_2 () : (Int, Int) { label exit : (Int,Int) { break exit (1,2); (3,4) } };

func foo_1_1 (x : Int) : Int {x + 1};

func foo_1_2 (x : Int) : (Int, Int) {(x,x)};

func foo_2_2 (x : Int, y : Int) : (Int, Int) {(x,y)};

func foo_2_1 (x : Int, y : Int) : Int {x + y};

foo_0_0();

assert (foo_0_1() == 1);

do {
let (x,y) = foo_0_2();
assert (x == 1);
assert (y == 2);
};

do {
let (x,y) = foo_0_return_2();
assert (x == 1);
assert (y == 2);
};

do {
let (x,y) = foo_0_break_2();
assert (x == 1);
assert (y == 2);
};

assert (foo_2_1(foo_1_2(5)) == 10);
