// test serialization of composite queries

actor This {

    public composite query func f() : async () {};

    public func go() : () {

       let blob = to_candid (This);

       // can deserialize as composite query
       let o1 = from_candid blob : ? (actor { f : composite query () -> async (); });
       assert o1 == ?This;

       // can't deserialize as query
       let o2 = from_candid blob : ? (actor { f : /* composite */ query () -> async (); });
       assert o2 == null;

       // can't deserialize as update
       let o3 = from_candid blob : ? (actor { f : /* composite  query */ () -> async (); });
       assert o3 == null;

       // can't deserialize as oneway
       let o4 = from_candid blob : ? (actor { f : /* composite  query */ () -> (); });
       assert o4 == null;

    };

}

//SKIP run
//SKIP run-ir
//SKIP run-low
//CALL ingress go "DIDL\x00\x00"
