type R1 = {
  a : { #a };
  b : { #b };
  c : { #c };
  d : { #d };
  e : { #e };
  f : { #f };
  g : { #g };
  h : { #h };
};

type R2 = R1 and {
  i : { #i };
  j : { #j };
  k : { #k };
  l : { #l };
  m : { #m };
  n : { #n };
  o : { #o };
  p : { #p };
  q : { #q };
  r : { #r };
  s : { #s };
  t : { #t };
};

func f (r : R1) : { #x } { r.x }; // unfolds R1

func g (r : R2) : { #x } { r.x }; // unfolds R2

func h (r : [R1]) : { #x } { r[0].x }; // both R1 and unfolding

func i (r : [R2]) : { #x } { r[0].x }; // both R2 and unfolding

func j (a : [()]) : { #x } { a.x }; // just array type
