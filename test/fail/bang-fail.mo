func wrong1() {
  let _ = (?1) ! : Nat; // reject in checking context
};

func wrong2() {
  let _ = ? () !; // not an option in checking
};

func wrong3() {
  let _ = (?1) !; // reject in synthesis
};

func wrong4() {
  let _ = ? (() !); // not an option in synthesis
};

func wrong5() {
  let _ = ? ((? ()) ! : Nat); // wrong content type
};
