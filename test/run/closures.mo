func add1 (x : Nat) : Nat = x + 1;
func add2 (x : Nat) : Nat = add1(x) + 1;
func add3 (x : Nat) : Nat = add2(x) + 1;

/* Storing functions */
let fs = [add1, add2, add3];

assert(fs[0](fs[1](fs[2](1))) == 7);

/* Closure (scalar) */
let answer = 42;
func test_answer(x : Nat) { assert (answer == x) };
test_answer(42);

/* Closure (heap object) */
let answers = [43, 44];
func test_answers(x : Nat) { assert (answers[1] == x) };
test_answers(44);

/* Closure (scalar, use-before-define) */
func test_answer2() { assert (answer2 == 42) };
let answer2 = 42;
test_answer2();

/* Closure (mutable scalar, use-before-define) */
var answer3 = 50;
func test_answer3() { assert (answer3 == 51) };
answer3 := 51;
test_answer3();
