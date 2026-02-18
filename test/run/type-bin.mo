//MOC-FLAG -A=M0194
let a : Int8 = 1;
let b = a + 1;
let c = 1 + a;
let d = a == (3 + 4*2**3);
let e = (5 - 2) * [(a, a)][0].1;
let f = (5 % 2) == (if true { a } else { a });
let g = 1 / (switch true { case false { a }; case true { a } });
let h = do? { (?a)! + 1 };
let i = 1 < (func f(x : Int8) : Int8 { x })(0);
let j = 0 != (do { let x : Int8 = 0; x });
let k = {x = a; b = true}.x ** 1;
let l = (switch a { case x x }) + 1;
let m = 1 - (object {public let y = [a]; public let x = y[0]}).x;
let o = (a, [a], {x=a}, ?a, ?a) != (0, [0, 0], {x=0}, ?0, null);
let p = (a + 1 + 2) == 0;
let q = 0 == 4 + 3 * 2**a + 2;
let z = (5 - 2) * (if true { [(a, a)][0].1 } else { let x : Int8 = 0; x });
