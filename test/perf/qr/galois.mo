/**
 * Module     : galois.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : Stable
 */

import Array "array";
import List "list";
import Nat "nat";
import Prelude "prelude";
import Prim "mo:⛔";
import Util "util"

module {

  type List<T> = List.List<T>;

  public let logs = [
    000, 001, 025, 002, 050, 026, 198, 003, 223, 051,
    238, 027, 104, 199, 075, 004, 100, 224, 014, 052,
    141, 239, 129, 028, 193, 105, 248, 200, 008, 076,
    113, 005, 138, 101, 047, 225, 036, 015, 033, 053,
    147, 142, 218, 240, 018, 130, 069, 029, 181, 194,
    125, 106, 039, 249, 185, 201, 154, 009, 120, 077,
    228, 114, 166, 006, 191, 139, 098, 102, 221, 048,
    253, 226, 152, 037, 179, 016, 145, 034, 136, 054,
    208, 148, 206, 143, 150, 219, 189, 241, 210, 019,
    092, 131, 056, 070, 064, 030, 066, 182, 163, 195,
    072, 126, 110, 107, 058, 040, 084, 250, 133, 186,
    061, 202, 094, 155, 159, 010, 021, 121, 043, 078,
    212, 229, 172, 115, 243, 167, 087, 007, 112, 192,
    247, 140, 128, 099, 013, 103, 074, 222, 237, 049,
    197, 254, 024, 227, 165, 153, 119, 038, 184, 180,
    124, 017, 068, 146, 217, 035, 032, 137, 046, 055,
    063, 209, 091, 149, 188, 207, 205, 144, 135, 151,
    178, 220, 252, 190, 097, 242, 086, 211, 171, 020,
    042, 093, 158, 132, 060, 057, 083, 071, 109, 065,
    162, 031, 045, 067, 216, 183, 123, 164, 118, 196,
    023, 073, 236, 127, 012, 111, 246, 108, 161, 059,
    082, 041, 157, 085, 170, 251, 096, 134, 177, 187,
    204, 062, 090, 203, 089, 095, 176, 156, 169, 160,
    081, 011, 245, 022, 235, 122, 117, 044, 215, 079,
    174, 213, 233, 230, 231, 173, 232, 116, 214, 244,
    234, 168, 080, 088, 175
  ];

  public func log(n : Nat) : Nat {
    let m = n % 256;
    if (m == 0) {
      Prelude.printLn("Error: Logarithm of zero is undefined in GF(256)!");
      Prelude.unreachable()
    };
    logs[m - 1]
  };

  public let alogs = [
    001, 002, 004, 008, 016, 032, 064, 128, 029, 058,
    116, 232, 205, 135, 019, 038, 076, 152, 045, 090,
    180, 117, 234, 201, 143, 003, 006, 012, 024, 048,
    096, 192, 157, 039, 078, 156, 037, 074, 148, 053,
    106, 212, 181, 119, 238, 193, 159, 035, 070, 140,
    005, 010, 020, 040, 080, 160, 093, 186, 105, 210,
    185, 111, 222, 161, 095, 190, 097, 194, 153, 047,
    094, 188, 101, 202, 137, 015, 030, 060, 120, 240,
    253, 231, 211, 187, 107, 214, 177, 127, 254, 225,
    223, 163, 091, 182, 113, 226, 217, 175, 067, 134,
    017, 034, 068, 136, 013, 026, 052, 104, 208, 189,
    103, 206, 129, 031, 062, 124, 248, 237, 199, 147,
    059, 118, 236, 197, 151, 051, 102, 204, 133, 023,
    046, 092, 184, 109, 218, 169, 079, 158, 033, 066,
    132, 021, 042, 084, 168, 077, 154, 041, 082, 164,
    085, 170, 073, 146, 057, 114, 228, 213, 183, 115,
    230, 209, 191, 099, 198, 145, 063, 126, 252, 229,
    215, 179, 123, 246, 241, 255, 227, 219, 171, 075,
    150, 049, 098, 196, 149, 055, 110, 220, 165, 087,
    174, 065, 130, 025, 050, 100, 200, 141, 007, 014,
    028, 056, 112, 224, 221, 167, 083, 166, 081, 162,
    089, 178, 121, 242, 249, 239, 195, 155, 043, 086,
    172, 069, 138, 009, 018, 036, 072, 144, 061, 122,
    244, 245, 247, 243, 251, 235, 203, 139, 011, 022,
    044, 088, 176, 125, 250, 233, 207, 131, 027, 054,
    108, 216, 173, 071, 142
  ];

  public func alog(n : Nat) : Nat {
    let m = n % 256;
    if (m == 255) {
      Prelude.printLn("Error: Antilogarithm of 255 is undefined in GF(256)!");
      Prelude.unreachable()
    };
    alogs[m]
  };

  public type Elem = { unbox : Nat };

  public func elemNew(n : Nat) : Elem {
    { unbox = n % 256 }
  };

  public func elemShow(elem : Elem) : Text {
    debug_show(elem.unbox)
  };

  public func elemToBit(elem : Elem) : Bool {
    elem.unbox > 0
  };

  public func elemFromBit(bit : Bool) : Elem {
    if (bit) ({ unbox = 1 }) else ({ unbox = 0 })
  };

  public func elemToBits(elem : Elem) : List<Bool> {
    Util.padLeftTo(8, Nat.natToBits(elem.unbox))
  };

  public func elemFromBits(bits : List<Bool>) : Elem {
    elemNew(Nat.natFromBits(bits))
  };

  public func elemEq(elem1 : Elem, elem2 : Elem) : Bool {
    elem1.unbox == elem2.unbox
  };

  public func elemAdd(elem1 : Elem, elem2 : Elem) : Elem {
    { unbox = Nat.natXor(elem1.unbox, elem2.unbox) }
  };

  public func elemSub(elem1 : Elem, elem2 : Elem) : Elem {
    elemAdd(elem1, elem2)
  };

  public func elemMul(elem1 : Elem, elem2 : Elem) : Elem {
    switch (elem1.unbox, elem2.unbox) {
      case (0, _) { elem1 };
      case (_, 0) { elem2 };
      case (a, b) { { unbox = alog((log(a) + log(b)) % 255) } }
    }
  };

  public func elemDiv(elem1 : Elem, elem2 : Elem) : Elem {
    switch (elem1.unbox, elem2.unbox) {
      case (_, 0) {
        Prelude.printLn("Error: Division by zero is undefined in GF(256)!");
        Prelude.unreachable()
      };
      case (0, _) { { unbox = 0 } };
      case (a, b) { { unbox = alog((255 + log(a) - log(b)) % 255) } }
    }
  };

  public func elemDivMod(elem1 : Elem, elem2 : Elem) : (Elem, Elem) {
    let elem3 = elemDiv(elem1, elem2);
    (elem3, elemSub(elem1, elem3))
  };

  public type Poly = { unbox : List<Elem> };

  public func polyNew(coeffs : [Nat]) : Poly {
    func step(n : Nat, accum : List<Elem>) : List<Elem> {
      List.push<Elem>(elemNew(n), accum)
    };
    let base = List.nil<Elem>();
    { unbox = Array.foldr<Nat, List<Elem>>(step, base, coeffs) }
  };

  public func polyShow(poly : Poly) : Text {
    switch (List.pop<Elem>(poly.unbox)) {
      case (null, _) { "[]" };
      case (?head, tail) {
        let base = elemShow(head);
        func step(elem : Elem, accum : Text) : Text {
          accum # "," # elemShow(elem)
        };
        "[" # List.foldLeft<Elem, Text>(tail, base, step) # "]"
      }
    }
  };

  public func polyToBits(poly : Poly) : List<Bool> {
    List.map<Elem, Bool>(poly.unbox, elemToBit)
  };

  public func polyFromBits(bits : List<Bool>) : Poly {
    { unbox = List.map<Bool, Elem>(bits, elemFromBit) }
  };

  public func polyLen(poly : Poly) : Nat {
    List.len<Elem>(poly.unbox)
  };

  public func polyTrim(poly : Poly) : Poly {
    func go(elems : List<Elem>) : List<Elem> {
      switch (List.pop<Elem>(elems)) {
        case (?{ unbox = 0 }, tail) { go(tail) };
        case _ { elems }
      }
    };
    { unbox = go(poly.unbox) }
  };

  public func polyOrder(poly : Poly) : Int {
    Prim.abs(polyLen(polyTrim(poly))) - 1
  };

  public func polyLeadCoeff(poly : Poly) : Elem {
    switch (List.pop<Elem>(polyTrim(poly).unbox).0) {
      case (?elem) { elem };
      case (null) { { unbox = 0 } }
    }
  };

  public func polyPadLeft(n : Nat, poly : Poly) : Poly {
    let zeros = List.replicate<Elem>(n, { unbox = 0 });
    { unbox = List.append<Elem>(zeros, poly.unbox) }
  };

  public func polyPadRight(n : Nat, poly : Poly) : Poly {
    let zeros = List.replicate<Elem>(n, { unbox = 0 });
    { unbox = List.append<Elem>(poly.unbox, zeros) }
  };

  public func polyGrow(to : Nat, poly : Poly) : Poly {
    let from = polyLen(poly);
    if (to > from) {
      polyPadLeft(to - from, poly)
    } else poly
  };

  public func polyZipWith(
    poly1 : Poly,
    poly2 : Poly,
    f : (Elem, Elem) -> Elem
  ) : Poly {
    let n1 = polyLen(poly1);
    let n2 = polyLen(poly2);
    let to = if (n1 > n2) n1 else n2;
    { unbox = List.zipWith<Elem, Elem, Elem>(
      polyGrow(to, poly1).unbox,
      polyGrow(to, poly2).unbox,
      f
    ) }
  };

  public func polyEq(poly1 : Poly, poly2 : Poly) : Bool {
    List.isEq<Elem>(polyTrim(poly1).unbox, polyTrim(poly2).unbox, elemEq)
  };

  public func polyAdd(poly1 : Poly, poly2 : Poly) : Poly {
    polyZipWith(poly1, poly2, elemAdd)
  };

  public func polySub(poly1 : Poly, poly2 : Poly) : Poly {
    polyAdd(poly1, poly2)
  };

  public func polyScale(alpha : Elem, poly : Poly) : Poly {
    func scale(elem : Elem) : Elem = elemMul(alpha, elem);
    { unbox = List.map<Elem, Elem>(poly.unbox, scale) }
  };

  public type Term = { coeff : Elem; order : Int };

  public func polyAddTerm(poly : Poly, term : Term) : Poly {
    let n = if (term.order <= 0) 0 else Prim.abs(term.order);
    polyAdd(poly, polyPadRight(n, polyNew([term.coeff.unbox])))
  };

  public func polyMulTerm(poly : Poly, term : Term) : Poly {
    let n = if (term.order <= 0) 0 else Prim.abs(term.order);
    polyScale(term.coeff, polyPadRight(n, poly))
  };

  public func polyDivMod(poly1 : Poly, poly2 : Poly) : (Poly, Poly) {
    let divisorLeadCoeff = polyLeadCoeff(poly2);
    let divisorOrder = polyOrder(poly2);
    func go(currentDividend : Poly, currentDivisor : Poly) : (Poly, Poly) {
      let currentOrder = polyOrder(currentDividend) - divisorOrder;
      if (currentOrder < 0) {
        (currentDivisor, currentDividend)
      } else {
        let currentDividendLeadCoeff = polyLeadCoeff(currentDividend);
        let currentCoeff = elemDiv(currentDividendLeadCoeff, divisorLeadCoeff);
        let currentTerm = { coeff = currentCoeff; order = currentOrder };
        let currentQuotient = polyMulTerm(poly2, currentTerm);
        let nextDividend = polySub(currentQuotient, currentDividend);
        let nextDivisor = polyAddTerm(currentDivisor, currentTerm);
        go(nextDividend, nextDivisor)
      }
    };
    go(poly1, polyNew([]))
  };

}
