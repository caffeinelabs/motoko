// test migration function restrictions

(with migration = ()) // reject, not a function
actor a {
};

(with migration = func <T>(x:T) : T {x}) // reject, a generic function
actor b {
};

(with migration = func () : {} {{}}) // reject, domain is not a record
actor c {
};

(with migration = func ({}) : () {}) // reject, co-domain is not a record
actor d {
};

(with migration = func ({f:()->()}) : () {}) // reject domain is unstable
actor e {
};

(with migration = func () : {f:()->()}{ {f = func(){}} }) // reject, co-domain is unstable
actor f {
   stable let f : Any = ()
};

(with migration = (func () : ({} -> {}) {}) ()) // reject, not static
actor g {
   stable let f : Any = ()
};

(with other = ()) // reject, no migration field
actor h {
};

({} with other = ())
actor i { // reject, no migration field
};

(with migration = func ({}) : {}{{}};
      extra = ()) // future: warn, unexpected field
actor j {
};

(with migration = func ({}) : {}{{}};) // error, unexpected module
module k {
};

(with migration = func ({}) : {}{{}};) // error, unexpected object
object l {
};
