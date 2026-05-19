//MOC-ENV MOC_UNLOCK_PRIM=yesplease

// Slice-3 smoke: parser + AST elaboration of `prim type`.
// Body is `prim switch` over a value-parameter (placeholder typed
// as Blob until slice 7 adds the dedicated Candid singleton).
// No call sites yet — slice 5/7 cover the surface usage.

prim type TyDesc<T>(stream : Blob) = prim switch (typCode(stream)) {
  case -3 : type T = Nat;
  case -4 : type T = Int;
  case _ : type T = Any;
};
