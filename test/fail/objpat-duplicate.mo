let o = {a = 1; b = 2};

switch o {
  case {a = x; b; a} {}
};

switch o {
  case {a; b = a} {}
};

// The duplicate field's variable is still bound
let {a = x; a = y} = o;
ignore (x + y);
