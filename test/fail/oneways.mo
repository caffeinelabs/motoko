// check explicit oneway declarations

actor a {

 public func ok1() : () {};
 public func ok2() : () = ignore ((async ()) : async ());

 public func wrong1() : () = ();
 public func wrong2() : () = ignore async ();
};

shared func warn1() {};
shared func warn2() = ignore ((async ()) : async ()) ;
shared func warn3() = ignore ((async return) : async ()) ;
shared func ok1() : () {};
shared func ok2() : () = ignore ((async ()) : async ()) ;
shared func ok3() : () = ignore ((async return) : async ()) ;
shared func wrong1() = ();
shared func wrong2() = ignore async ();
