import c "empty";
actor class (c : Text) {
  assert (c == "Hello"); // would crash if compiled import of c captured parameter c
};
