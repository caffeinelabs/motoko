import Prim "mo:⛔";

actor a {

  public query func ok(): async () {
    throw Prim.error("abcdefgh");
  };
  public query func crash(): async () {
    throw Prim.error("abcdefgh" # "i");
  };
};

ignore a.ok(); //OR-CALL query ok "DIDL\x00\x00"
ignore a.crash(); //OR-CALL query crash "DIDL\x00\x00"
