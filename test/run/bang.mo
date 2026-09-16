import Prim "mo:⛔";

func print(o : ? Nat) { Prim.debugPrint(debug_show(o));};

let o1 = do ? {
    let oi = ?1;
    let oj = ?2;
    oi! + oj!;
};
print(o1);
assert (o1 == ? 3);


let o2 = do ? {
   let oi = ?1;
   let oj : ?Nat = null;
   oi! + oj!;
};
print(o2);
assert (o2 == null);


let o3 = do ? {
   var sum = 0;
   for(o in [?1, ?2, ?3].values()) {
     sum += o!
   };
   sum
};
print(o3);

assert (o3 == ? 6);
let o3Values = do ? {
   var sum = 0;
   for(o in [?1, ?2, ?3].values()) {
     sum += o!
   };
   sum
};
assert (o3Values == ? 6);

let o4 = do ? {
   var sum = 0;
   for(o in [?1, ?2, null].values()) {
     sum += o!
   };
   sum
};
print o4;
assert (o4 == null);

let o4Values = do ? {
   var sum = 0;
   for(o in [?1, ?2, null].values()) {
     sum += o!
   };
   sum
};
assert (o4Values == null);

/* nesting */

let o5 = do ? {
   let o = ??0;
   o!!
};
print o5;
assert (o5 == ?0);

let o6 : ? Nat = do ? {
   let o = ?null;
   o!!
};
print o6;
assert (o6 == null);

let o7 = do ? {
   let o = null : ? None;
   o!!
};
print o7;
assert (o7 == null);
