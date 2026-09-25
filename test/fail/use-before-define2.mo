// This line is fine (no use-before-define)
let f1 = do { func f():Int = x };
// This is a false positive, due to the unsaturated occurence of f
// A more elaborate dependency analysis in freevars.ml could catch this.
let f2 = do { let f3 = func ():Int = x; f3 };
let x = 1;
let y = f1() + f2();
assert(y == 1);
