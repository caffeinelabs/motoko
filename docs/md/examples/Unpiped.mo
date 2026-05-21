import List "mo:core/pure/List";
import Nat "mo:core/Nat";

{ multiples =
   List.filter<Nat>(
     List.fromIter(Nat.range(0, 10)),
     func n { n % 3 == 0 }) };
