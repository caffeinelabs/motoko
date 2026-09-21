import Prim "mo:⛔";
import L "list";
import T "types";

module {

 public type ClientData = {
   id : Nat;
   client : shared Text -> ();
   var revoked : Bool;
 };

 public persistent actor class Server() = {
   transient var nextId : Nat = 0;
   transient var clients : L.List<ClientData> = null;

   public func subscribe(aclient : shared Text -> ()) : async T.Subscription {
     let c = {id = nextId; client = aclient; var revoked = false};
     nextId += 1;
     let cs = {head = c; var tail = clients};
     clients := ?cs;
     return object {
       public shared func post(message : Text) {
	 if not c.revoked {
	   let id = c.id;
	   var next = clients;
	   label sends loop {
	     switch next {
	       case null { break sends }
	       case (?n) {
		 if n.head.id != id { n.head.client(message) };
		 next := n.tail;
	       }
	     };
	   };
	 }
       };
       public shared func cancel() { unsubscribe(c.id) };
     };
   };

   func unsubscribe(id : Nat) {
     var prev : L.List<ClientData> = null;
     var next = clients;
     loop {
       switch next {
	 case null { return }
	 case (?n) {
	   if n.head.id == id {
	     switch prev {
	       case null { clients := n.tail }
	       case (?p) { p.tail := n.tail }
	     };
	     Prim.debugPrint "(unsubscribe "; Prim.debugPrintInt id; Prim.debugPrint ")\n";
	     return;
	   };
	   prev := next;
	   next := n.tail;
	 }
       };
     };
   };
 };

}
