actor class A() : async actor {} {}; //ok

actor class A1<T>() {}; // reject generic actor class
actor class A2<T, U>() {}; // reject generic actor class

actor class A3(c:[var Int]) {}; // reject non-shared parameter

actor class A4() : async object {} {}; // reject bad async return type
actor class A5() : actor {} {}; // reject non async return type
