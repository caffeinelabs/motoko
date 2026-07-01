// Exercise incompatible_obj_sorts: Object vs Module sort mismatch.
// The ObjSort type has: Object | Module | Actor | Memory | Mixin.
// Mismatch between any two triggers incompatible_obj_sorts in rel_typ.

// Create a module-type expression and try to use it as an object type
// The type annotation forces an Object type but the value has Module type
module M { public let x : Nat = 42; };

// Module used where Object expected — sort mismatch
let _ : {x : Nat} = M;
