import Map "mo:core/Map";
import Set "mo:core/Set";
import Nat "mo:core/Nat";
import Iter "mo:core/Iter";
import Order "mo:core/Order";
import Text "mo:core/Text";
import Array "mo:core/Array";
import List "mo:core/List";
import Stack "mo:core/Stack";
import Queue "mo:core/Queue";
import PureMap "mo:core/pure/Map";
import PureSet "mo:core/pure/Set";
import PureList "mo:core/pure/List";
import PureQueue "mo:core/pure/Queue";

// custom self.view(...) methods, collected in a mixin for convenience.
mixin () {

  module MapView {
   public func view<K,V>(self : Map.Map<K, V>, compare : (implicit : (K,K) -> Order.Order)) : (ko : ?K, count : ?Nat) -> [(K, V)] =
      func (ko, count) {
        let entries = switch ko {
      	  case null {
            self.entries()
          };
          case (?k) {
          self.entriesFrom(k)
          };
        };
        switch count {
          case null { entries.toArray() };
          case (?c) { entries.take(c).toArray() };
        };
     }
  };

  module SetView {

   public func view<K>(
     self : Set.Set<K>,
     compare : (implicit : (K,K) -> Order.Order)) : (
     ko : ?K,
     count : ?Nat) -> [K] =
     func (ko, count) {
      let entries = switch ko {
        case null {
          self.values()
        };
        case (?k) {
          self.valuesFrom(k)
        };
      };
      switch count {
        case null { entries.toArray() };
        case (?c) { entries.take(c).toArray() };
      };
    };
  };

  module ArrayView {

   public func view<V>(self : [V]) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module VarArrayView {

   public func view<V>(self : [var V]) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module ListView {

   public func view<V>(self : List.List<V>) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module StackView {

   public func view<V>(self : Stack.Stack<V>) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module QueueView {

   public func view<V>(self : Queue.Queue<V>) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           self.values()
         };
         case (?io) {
           self.values().drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  // ── Pure (immutable) collection viewers ──────────────────────────
  // PureMap/PureSet use `dropWhile` with `compare` for key-based starts.
  // PureList/PureQueue use index-based starts via iterator `.drop()`.

  module PureMapView {

   public func view<K,V>(self : PureMap.Map<K, V>, compare : (implicit : (K,K) -> Order.Order)) :
     (ko : ?K, count : ?Nat) -> [(K, V)] =
     func (ko, count) {
       let entries = switch ko {
         case null {
           PureMap.entries(self)
         };
         case (?k) {
           PureMap.entries(self).dropWhile(
             func((ek, _) : (K, V)) : Bool { compare(ek, k) == #less })
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module PureSetView {

   public func view<K>(self : PureSet.Set<K>, compare : (implicit : (K,K) -> Order.Order)) :
     (ko : ?K, count : ?Nat) -> [K] =
     func (ko, count) {
       let entries = switch ko {
         case null {
           PureSet.values(self)
         };
         case (?k) {
           PureSet.values(self).dropWhile(
             func(ek : K) : Bool { compare(ek, k) == #less })
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module PureListView {

   public func view<V>(self : PureList.List<V>) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           PureList.values(self)
         };
         case (?io) {
           PureList.values(self).drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

  module PureQueueView {

   public func view<V>(self : PureQueue.Queue<V>) :
     (io : ?Nat, count : ?Nat) -> [V] =
     func (io, count) {
       let entries = switch io {
         case null {
           PureQueue.values(self)
         };
         case (?io) {
           PureQueue.values(self).drop(io)
         };
       };
       switch count {
         case null { entries.toArray() };
         case (?c) { entries.take(c).toArray() };
       };
    };
  };

}
