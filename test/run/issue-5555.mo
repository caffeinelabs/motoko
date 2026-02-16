//MOC-FLAG -dl

ignore 57 + 0;
ignore 42 + -0;
ignore 25 * 1;
ignore 25 * +1;
ignore (42 - 0) : Nat;
ignore 42 - 0;
ignore 42 - -0;
ignore 25 / 1;
ignore 25 / +1;
ignore 0 + 42;
ignore -0 + 42;
ignore 1 * 25;
ignore +1 * 25;

ignore 42 : Int + 0;
ignore 42 : Nat8 + 0;

ignore 42 + 0 : Nat16;
ignore 42 + 0 : Nat32;
ignore 42 + 0 : Nat64;

ignore 37 | 0 : Nat64;

ignore 25 : Nat8 * 1;
ignore 25 * 1 : Nat16;
ignore 25 * 1 : Nat32;
ignore 25 * 1 : Nat64;

//SKIP run
//SKIP run-ir
//SKIP run-low
//FILTER comp tail -n 27
//SKIP wasm-run
