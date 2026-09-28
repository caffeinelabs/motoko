// test some forms of typed labels, aka. outward/non-capturable (2nd class) continuations

let a0 = label c : Nat { loop { break c 42 }; 25 };
let b0 = label c : Nat { loop { break c 42 } while true; 25 };
let c0 = label c : Nat { while true { break c 42 }; 25 };
let d0 = label c : Nat { for _ in "HEY!".chars() { break c 42 }; 25 };
let e0 = label c : Nat { break c 42; 25 };

let a1 = label c : Nat loop { break c 42 };
let b1 = label c : Nat loop { break c 42 } while true;
let c1 = label c : Nat while true { break c 42 };
let d1 = label c : Nat for _ in "HEY!".chars() { break c 42 };
let e1 = label c : Nat break c 42;

let a2 = label c : Nat { loop { continue c }; 25 }; // c is not a continue label
let b2 = label c : Nat { loop { continue c } while true; 25 }; // ditto
let c2 = label c : Nat { while true { continue c }; 25 }; // ditto
let d2 = label c : Nat { for _ in "HEY!".chars() { continue c }; 25 }; // ditto
let e2 = label c : Nat { continue c }; // ditto

let a3 = label c : Nat loop { continue c };
let b3 = label c : Nat loop { continue c } while true;
let c3 = label c : Nat while true { continue c };
let d3 = label c : Nat for _ in "HEY!".chars() { continue c };
let e3 = label c : Nat continue c; // c is not a continue label
