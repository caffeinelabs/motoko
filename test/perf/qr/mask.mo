/**
 * Module     : mask.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : Experimental
 */

import Common "common";
import List "list";
import Option "option";
import Symbol "symbol";
import Version "version";

module {

  type Candidate = ([var [var Bool]], List<Bool>);
  type ErrorCorrection = Common.ErrorCorrection;
  type List<T> = List.List<T>;
  type Version = Version.Version;

  public func generate(
    version : Version,
    level : ErrorCorrection,
    data : List<Bool>
  ) : ([var [var Bool]], List<Bool>) {

    let tests = List.fromArray<(Nat, Nat) -> Bool>([
      maskTest000,
      maskTest001,
      maskTest010,
      maskTest011,
      maskTest100,
      maskTest101,
      maskTest110,
      maskTest111
    ]);

    let masks = List.map<(Nat, Nat) -> Bool, List<Bool>>(tests, func (test) {
      let w = Common.width(version);
      func mirror(n : Nat) : Nat { w - n - 1 };
      List.map<(Nat, Nat), Bool>(Symbol.pathCoords(version), func (i, j) {
        test(mirror(i), mirror(j))
      })
    });

    let matrices = List.map<List<Bool>, [var [var Bool]]>(masks, func (mask) {
      Symbol.symbolize(
        version,
        List.zipWith<Bool, Bool, Bool>(mask, data, func (x, y) { x != y })
      )
    });

    let maskRefs = List.fromArray<List<Bool>>([
      maskRef000,
      maskRef001,
      maskRef010,
      maskRef011,
      maskRef100,
      maskRef101,
      maskRef110,
      maskRef111
    ]);

    let candidates = List.zip<[var [var Bool]], List<Bool>>(
      matrices,
      maskRefs
    );

    // TODO: Score candidates.
    Option.unwrap<Candidate>(List.nth<Candidate>(candidates, 3))
  };

  let maskRef000 = ?(false, ?(false, ?(false, null)));
  let maskRef001 = ?(false, ?(false, ?(true, null)));
  let maskRef010 = ?(false, ?(true, ?(false, null)));
  let maskRef011 = ?(false, ?(true, ?(true, null)));
  let maskRef100 = ?(true, ?(false, ?(false, null)));
  let maskRef101 = ?(true, ?(false, ?(true, null)));
  let maskRef110 = ?(true, ?(true, ?(false, null)));
  let maskRef111 = ?(true, ?(true, ?(true, null)));

  func maskTest000(i : Nat, j : Nat) : Bool { (i + j) % 2 == 0 };
  func maskTest001(i : Nat, j : Nat) : Bool { i % 2 == 0 };
  func maskTest010(i : Nat, j : Nat) : Bool { j % 3 == 0 };
  func maskTest011(i : Nat, j : Nat) : Bool { (i + j) % 3 == 0 };
  func maskTest100(i : Nat, j : Nat) : Bool { ((i / 2) + (j / 3)) % 2 == 0 };
  func maskTest101(i : Nat, j : Nat) : Bool { (i * j) % 2 + (i * j) % 3 == 0 };
  func maskTest110(i : Nat, j : Nat) : Bool { ((i * j) % 2 + (i * j) % 3) % 2 == 0 };
  func maskTest111(i : Nat, j : Nat) : Bool { ((i * j) % 3 + (i + j) % 2) % 2 == 0 };

}
