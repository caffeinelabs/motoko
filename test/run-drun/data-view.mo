//MOC-FLAG --package core /home/crusso/motoko-core/src
import Map "mo:core/Map";
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";
import Order "mo:core/Order";
import Text "mo:core/Text";

// curried version of data-view.mo

persistent actor {

  module MapView {

   public func view<K,V>(self : Map.Map<K, V>, compare : (implicit : (K,K) -> Order.Order)) : (ko : ?K, count : Nat) -> [(K, V)]=
      func (ko, count) {
        let entries = switch ko {
      	  case null {
            self.entries()
          };
          case (?k) {
          self.entriesFrom(k)
          };
        };
        entries.take(count).toArray();
     }
  };

  let map : Map.Map<Nat, Text> = Map.empty();

  public query func mapView(ko: ?Nat, count: Nat) : async [(Nat, Text)] {
     map.view(/*Nat.compare*/)(ko, count);
  };

  module SetView {

   public func view<K>(
     self : Set.Set<K>,
     compare : (implicit : (K,K) -> Order.Order)) : (
     ko : ?K,
     count : Nat) -> [K] =
     func (ko, count) {
      let entries = switch ko {
        case null {
          self.values()
        };
        case (?k) {
          self.valuesFrom(k)
        };
      };
      entries.take(count).toArray();
    };
  };

  let set : Set.Set<Nat> = Set.empty();

  public query func setView(ko: ?Nat, count: Nat) : async [Nat] {
     set.view(/*Nat.compare*/)(ko, count);
  };

  module ArrayView {

   public func view<V>(self : [V]) :
     (io : ?Nat, count : Nat) -> [V] =
     func (io, count) {
       // TODO: use slice instead
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       entries.take(count).toArray();
    };
  };

  let array : [(Nat, Text)] = [];

  public query func arrayView(ko: ?Nat, count: Nat) : async [(Nat, Text)] {
     array.view()(ko, count);
  };


  // here, [insible_array.view] produces a non-shared (mutable) type, omit viewer
  // later maybe approximate by shared type.
  let invisible_array : [[var Nat]] = [];

  // shared values we can just display, sans viewer
  var some_variant = #node (#leaf, 0, #leaf);
  let some_record = {a=1;b ="hello"; c = true} ;

  // stable, non-shared values we can't just display in full, without viewer
  let some_mutable_record = {var a = 1};


}

