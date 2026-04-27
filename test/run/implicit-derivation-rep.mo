//MOC-FLAG --package core $MOTOKO_CORE


import Array "mo:core/Array";

func absurd() : None { absurd() };

// try to implement generic toCandid/fromCandid functions using
// embedding/projection pairs via candid
// (not a practical example as it stands)

type Rep<A> = {
  inj: A -> Blob;
  proj: Blob -> A
};

module NatRep {
  public let rep : Rep<Nat> =
    { inj = func v {
        to_candid (v)
      };
      proj = func b {
        let ?v = from_candid b : ?Nat else absurd();
        v
      }
    };
};

module IntRep {
  public let rep : Rep<Int> =
    { inj = func v {
        to_candid (v)
      };
      proj = func b {
        let ?v = from_candid b : ?Int else absurd();
        v
      }
    };
};

module BoolRep {
  public let rep : Rep<Bool> =
    { inj = func v {
        to_candid (v)
      };
      proj = func b {
        let ?v = from_candid b : ?Bool else absurd();
	v;
      };
    };
};

module PairRep {
  public func rep<A, B>(
    repA : (implicit : (rep : Rep<A>)),
    repB : (implicit : (rep : Rep<B>)))
    : Rep<(A,B)> = {
      inj = func p { to_candid (repA.inj (p.0), repB.inj (p.1)) };
      proj = func b {
        let ?(b0, b1) = from_candid b : ?(Blob,Blob) else absurd();
        (repA.proj b0, repB.proj b1)
      }
    };
};

module ArrayRep {
  public func rep<A>(
    repA : (implicit : (rep : Rep<A>))) :
    Rep<[A]> = {
    inj = func as {
      let bs = to_candid (as.map(repA.inj));
      to_candid (bs)
    };
    proj = func b {
      let ?bs = from_candid b : ?[Blob] else absurd();
      bs.map(repA.proj)
    }
   }
};

do {
  func toCandid<A>(rep : (implicit : (rep : Rep<A>)), a : A) : Blob = rep.inj a;
  func fromCandid<A>(rep : (implicit : (rep : Rep<A>)), b : Blob) : A = rep.proj b;

  do { // works
    let b = toCandid<Nat>(0);
    let a = fromCandid<Nat>(b) : Nat;
  };

  do {
    let b = toCandid(0);
    let a = fromCandid(b) : Nat; // fails inference
  };

  do { // fails inference
    let b = toCandid<[Nat]>([0]) : Blob;
    let a = fromCandid<[Nat]>(b) : [Nat];
  };


  do { // fails inference
    let b = toCandid<[(Nat,Bool)]>([(0,true)]);
    let a = fromCandid<[(Nat,Bool)]>(b) : [(Nat,Bool)];
  };
}

