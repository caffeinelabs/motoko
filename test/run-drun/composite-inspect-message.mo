//MOC-FLAG -A=M0194
// NB: inspect cannot be tested in drun.
//     (requires dfx or playground for HTTP interface)
// tests composite queries excluded from message inspection
actor {

   var c = 0;

   public func inc() : async () { c += 1 };
   public func set(n : Nat) : async () { c := n };
   public query func read() : async Nat { c };
   public func reset() : () { c := 0 }; // oneway
   public composite query func comp() : async Nat { c }; // not to be included in domain of `inspect` as composite_query cannot be used in update calls (unlike queries)


   system func inspect(
     {
       caller : Principal; // unused, could be omitted
       arg : Blob;
       msg : {
         #inc : Any;
         #set : () -> Nat;
         #read : Any;
         #reset : () -> ();
       }
     }) : Bool {
    if (arg.size() > 512) return false;
    switch (msg) {
      case (#set n) { n() != 13 };
      case (#reset f) { false };
      case _ { true }; // allow inc and read
    }
  }
};
