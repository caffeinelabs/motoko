// check explicit oneway declarations

actor a {

 public func ok1() : () {};
 public func ok2() : () = ();

 public func wrong1() : () = ignore ((async ()) : async ());
 public func wrong2() : () = ignore async ();
};

shared func warn1() {};
shared func warn2() = ();
shared func ok1() : () {};
shared func ok2() : () = ();
shared func wrong1() : () = ignore ((async ()) : async ()) ;
shared func wrong2() : () = ignore ((async return) : async ()) ;
shared func wrong3() = ignore async ();
