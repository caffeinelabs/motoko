import Nat "mo:core/Nat";
import Iter "mo:core/Iter";
import List "mo:core/pure/List";

Nat.range(0, 10) |>
  List.fromIter _ |>
    List.filter<Nat>(_, func n { n % 3 == 0 }) |>
      { multiples = _ };
