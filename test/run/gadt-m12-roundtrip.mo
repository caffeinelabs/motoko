// M12: to_candid + from_candid round-trip on a refinement-only GADT.
// The wire format is monomorphised on both sides — the codegen's
// type_desc pass and mo_to_idl both emit `Expr<Bool>` as the pruned
// `{#bool, #if_}` variant (no `#int` arm). Encoder and decoder agree.

type Expr<A> = {
  #int  : type A = Nat in Nat;
  #bool : type A = Bool in A;
  #if_  : (Expr<Bool>, Expr<A>, Expr<A>);
};

// Refinement-only arm.
let e1 : Expr<Bool> = #bool true;
let back1 : ?Expr<Bool> = from_candid (to_candid (e1));
switch back1 {
  case (?(#bool b)) assert b == true;
  case _ assert false;
};

// Recursive `#if_` arm, also refinement-only.
let e2 : Expr<Bool> = #if_(#bool true, #bool false, #bool true);
let back2 : ?Expr<Bool> = from_candid (to_candid (e2));
switch back2 {
  case (?(#if_(c, _, _))) {
    switch c {
      case (#bool b) assert b == true;
      case _ assert false;
    };
  };
  case _ assert false;
};

//SKIP run
//SKIP run-ir
//SKIP run-low
