//MOC-FLAG -A=M0194
import Prim "mo:⛔";

actor {
    transient let ?x = (null : ?Nat) else { Prim.trap "x was null" };
};
