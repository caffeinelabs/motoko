// Region types are stable, but not shared.

func badEquals(r1: Region, r2 : Region) : Bool {
  r1 == r2 // reject
};

func badNotEquals(r1 : Region, r2 : Region) : Bool {
  r1 != r2 // reject
};

func badEqualsNested(r1: Region, r2 : Region) : Bool {
  ?r1 == ?r2 // reject
};

func badNotEqualsNested(r1 : Region, r2 : Region) : Bool {
  ?r1 != ?r2 // reject
};

func ToCandid(r : Region) : Blob {
  to_candid(r); // reject
};

func FromCandid(b : Blob) : ?Region {
  let r: ?Region = from_candid b; // reject
};

do {
  type t = async Region; // reject
};
