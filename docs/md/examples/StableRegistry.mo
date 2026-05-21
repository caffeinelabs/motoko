import Text "mo:core/Text";
import Map "mo:core/Map";
import Iter "mo:core/Iter";

persistent actor Registry {

  var entries : [(Text, Nat)] = []; // implicitly `stable`

  let map = Map.fromIter<Text,Nat>(
    entries.values(),
    Text.compare
  );

  public func register(name : Text) : async () {
    switch (Map.get(map, Text.compare, name)) {
      case null  {
        Map.add(map, Text.compare, name, Map.size(map));
      };
      case (?_) { };
    }
  };

  public func lookup(name : Text) : async ?Nat {
    Map.get(map, Text.compare, name);
  };

// Using preupgrade is discouraged and should be avoided if possible.
  system func preupgrade() {
    entries := Iter.toArray(Map.entries(map));
  };

// Using postupgrade is discouraged and should be avoided if possible.
  system func postupgrade() {
    entries := [];
  };
}
