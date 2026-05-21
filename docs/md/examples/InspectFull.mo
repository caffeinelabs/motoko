import Principal = "mo:core/Principal";

persistent actor {

   var c = 0;

   public func inc() : async () { c += 1 };
   public func set(n : Nat) : async () { c := n };
   public query func read() : async Nat { c };
   public func reset() : () { c := 0 }; // oneway

   system func inspect(
     {
       caller : Principal;
       arg : Blob;
       msg : {
         #inc : () -> ();
         #set : () -> Nat;
         #read : () -> ();
         #reset : () -> ();
       }
     }) : Bool {
    if (Principal.isAnonymous(caller)) return false;
    if (arg.size() > 512) return false;
    switch (msg) {
      case (#inc _) { true };
      case (#set n) { n() != 13 };
      case (#read _) { true };
      case (#reset _) { false };
    }
  }
};
