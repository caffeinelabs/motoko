//MOC-FLAG --enhanced-orthogonal-persistence --default-persistent-actors
import Prim "mo:prim";

actor {
   type Node<K, V> = ?{ key : K; value : V; var next : Node<K, V> };
   type List<K, V> = {
      var first : Node<K, V>;
      var last : Node<K, V>;
   };
   let list : List<Nat, Text> = {
      var first = null;
      var last = null;
   };
   var counter = 0;

   Prim.debugPrint("INITIALIZED after graph-copy: " # debug_show (counter));

   func insert<K, V>(list : List<K, V>, key : K, value : V) {
      let node : Node<K, V> = ?{ key; value; var next = null };
      switch (list.last) {
         case null {
            list.first := node;
         };
         case (?previous) {
            previous.next := node;
         };
      };
      list.last := node;
   };

   public func populate() : async () {
      let limit = counter + 10;
      while (counter < limit) {
         insert(list, counter, debug_show (counter));
         counter += 1;
      };
   };

   public func print() : async () {
      var current = list.first;
      loop {
         switch current {
            case null return;
            case (?node) {
               Prim.debugPrint(debug_show (node.key) # " " # node.value);
               current := node.next;
            };
         };
      };
   };
};
