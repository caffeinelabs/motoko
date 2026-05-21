import Text "mo:core/Text";
import Map "mo:core/Map";

persistent actor Registry {

  let map = Map.empty<Text, Nat>();

  public func register(name : Text) : async () {
    switch (Map.get(map, Text.compare, name)) {
      case null {
        Map.add(map, Text.compare, name, Map.size(map));
      };
      case (?_) { };
    }
  };

  public func lookup(name : Text) : async ?Nat {
    Map.get(map, Text.compare, name);
  };
};

await Registry.register("hello");
(await Registry.lookup("hello"), await Registry.lookup("world"))
