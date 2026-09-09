// Provides provenance for the committed classical `old.wasm` fixture used by
// `migrate-stabilization.drun` (built from this file by moc 1.14.1, see note.txt).
// Not compiled by the test runner.
import Prim "mo:prim";

actor {
   type Node<K, V> = ?{ key : K; value : V; var next : Node<K, V> };
   type List<K, V> = {
      var first : Node<K, V>;
      var last : Node<K, V>;
   };
   stable let list : List<Nat, Text> = {
      var first = null;
      var last = null;
   };
   stable var counter = 0;

   Prim.debugPrint("INITIALIZED: " # debug_show(counter));

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
