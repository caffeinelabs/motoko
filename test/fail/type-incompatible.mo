// Exercise incompatible_* error reporting in type.ml by triggering subtype failures
// that produce detailed error explanations.
// These hit: incompatible_types, incompatible_prims, incompatible_obj_sorts,
// incompatible_func_sorts, incompatible_func_controls, incompatible_funcs,
// incompatible_async_sorts, incompatible_async_scopes.

// incompatible_prims: Nat not <: Text (distinct primitive types)
let _ : Text = (42 : Nat);
