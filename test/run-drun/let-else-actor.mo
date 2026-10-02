//MOC-FLAG -A=M0194
import Prim "mo:⛔";

actor {
    transient let none : ?Nat = null;
    transient let ?x = none else { Prim.trap "x was null" };
};
