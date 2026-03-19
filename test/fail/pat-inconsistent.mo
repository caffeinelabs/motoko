// Okay joins
/*let _ = func(1 or 2) {};
let _ = func(1 or -1) {};
let _ = func(1 or (_ : Int)) {};
let _ = func((1 : Int) or 2) {};
let _ = func({a = _ : Int} or {a = _ : Nat}) {};
let _ = func((_ : Int -> Int) or (_ : Nat -> Nat)) {};
let _ = func((-1, 2) or (2, -1)) {};
let _ = func(#A or #A) {};
let _ = func(#A or #A()) {};
let _ = func(#A or #A(_ : ())) {};
let _ = func(#A(5) or #A(_ : Int)) {};
let _ = func(#A or #B(_ : Nat)) {};

// Bogus joins
do { let _ = func(1 or 1.0) {} };
do { let _ = func(1 or "") {} };
do { let _ = func(true or "") {} };
do { let _ = func(1 or ()) {} };
do { let _ = func(1 or {}) {} };
do { let _ = func(1 or (_ : Text)) {} };
do { let _ = func({a = _ : Int} or {b = _ : Nat}) {} };
do { let _ = func((_ : Int8) or (_ : Nat8)) {} };
do { let _ = func(("", 1) or ("", 1.0)) {} };
do { let _ = func((_ : Int16, "") or (_ : Nat16, "")) {} };
do { let _ = func(#A or #A("")) {}; };
do { let _ = func(#A(0) or #A("")) {}; };

do { let _ = func(1 or (_ : Any)) {} };
do { let _ = func((_ : Any) or 1.4) {} };
do { let _ = func(() or (_ : Any)) {} };
do { let _ = func<A>("" or (_ : A)) {} };
do { let _ = func((_ : Any) or (_ : [Nat])) {} };
do { let _ = func((_ : Any) or (_ : () -> ())) {} };
do { let _ = func(("", 1) or ("", _ : Any)) {} };
do { let _ = func(#A or #A(_ : Any)) {}; };
do { let _ = func(#A(5) or #A(_ : Any)) {}; };*/

// No coverage check for ill-typed cases
switch (#A : {#A}) {
  case (#A) {};
  case ({B = _}) {};
  case _ {};
  case _ {};
};

switch (true : Bool) {
  case true {};
  case 1 {};
  case false {};
};

switch (true : Bool) {
  case (#geese) {};
};

// Coverage check for disjoint variants
switch (#sparrows : { #sparrows }) {
  case (#geese) {};
};

func absurd(birds : {#}) =
  switch birds {
    case (#geese) {};
  };

func bad_pat() {
  let (_, _) = 10;
};

func nested_bad_pat() {
  let ((_, _), _) = (10, 20);
};
