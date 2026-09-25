type L = { a : {} };
type U = {};

let l = { a = {} };

func f(l : L) : U = switch l {
  case (u : U) { u }
};

let {} = f(l);

// tuples
func t1(tl : (L, L)) : (U, U) = switch tl {
  case (tu : (U, U)) { tu }
};

let ({}, {}) = t1((l, l));

func t2(tl : (L, L)) : (U, U) = switch tl {
  case (u1 : U, u2 : U) { (u1, u2) }
};

let ({}, {}) = t2((l, l));

// options

func o1(ol : ? L) : ? U = switch ol {
  case (null : ? U) { null }
  case (ou: ? U) { ou }
};

let (? {} or _) = o1(? l);

func o2(tl : ? L) : ? U = switch tl {
  case (null : ? U) { null }
  case ? u { (? u) }
};

let (? {} or _) = o2(? l);

// records

func r1(rl : { a : L }) : { a : U } = switch rl {
  case (ru : { a : U }) { ru }
};

let { a = {} } : { a : U } = r1({ a = l });

func r2(rl : { a : L }) : { a : U } = switch rl {
  case { a = u : U } { ({ a = u }) }
};

let { a = {} } : { a : U } = r2({ a = l });

func r3(rl : { a : L }) : {} = switch rl {
  case {} { ({}) }
};

let {} : {} = r3({ a = l });

// variants

func v1(vl : { #a : L }) : { #a : U } = switch vl {
  case (vu : { #a : U }) { vu }
};

let (#a {})= v1(#a l);

func v2(vl : { #a : L }) : { #a : U } = switch vl {
  case #a(u) { #a u }
};

let (#a {}) = v2(#a l);

// alternative patterns

func a(l : { a : Int }):U = switch l {
  case (({ a = 1 } : { a : Int }) or (_ : U)) { l }
};

let {} = a({ a = 2 });
