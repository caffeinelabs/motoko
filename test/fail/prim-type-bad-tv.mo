//MOC-ENV MOC_UNLOCK_PRIM=yesplease

// Negative: arm refines `Z`, but the alias has no parameter `Z`.
prim type Bad<T>(stream : Blob) = prim switch (typCode(stream)) {
  case -3 : type Z = Nat;
};
