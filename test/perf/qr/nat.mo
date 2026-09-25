/**
 * Module     : nat.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : Stable
 */

import List "list";
import Prim "mo:⛔";

module {

  type List<T> = List.List<T>;

  public func natNot(a : Nat) : Nat {
    natMap(a, func (x) { ^ x })
  };

  public func natAnd(a : Nat, b : Nat) : Nat {
    natZipWith(a, b, func (x, y) { x & y })
  };

  public func natOr(a : Nat, b : Nat) : Nat {
    natZipWith(a, b, func (x, y) { x | y })
  };

  public func natXor(a : Nat, b : Nat) : Nat {
    natZipWith(a, b, func (x, y) { x ^ y })
  };

  public func natMap(a : Nat, f : Nat8 -> Nat8) : Nat {
    natFromBytes(List.map<Nat8, Nat8>(natToBytes(a), f))
  };

  public func natZipWith(a : Nat, b : Nat, f : (Nat8, Nat8) -> Nat8) : Nat {
    var xs = natToBytes(a);
    var ys = natToBytes(b);
    let xsLen = List.len<Nat8>(xs);
    let ysLen = List.len<Nat8>(ys);
    if xsLen < ysLen {
      xs := List.append<Nat8>(List.replicate<Nat8>(ysLen - xsLen, 0), xs);
    };
    if xsLen > ysLen {
      ys := List.append<Nat8>(List.replicate<Nat8>(xsLen - ysLen, 0), xs);
    };
    let zs = List.zipWith<Nat8, Nat8, Nat8>(xs, ys, f);
    let c = natFromBytes(zs);
    c
  };

  public func natToBytes(n : Nat) : List<Nat8> {
    var a = 0;
    var b = n;
    var bytes = List.nil<Nat8>();
    var test = true;
    while test {
      a := b % 256;
      b := b / 256;
      bytes := List.push<Nat8>(Prim.natToNat8(a), bytes);
      test := b > 0;
    };
    bytes
  };

  public func natFromBytes(bytes : List<Nat8>) : Nat {
    var n = 0;
    var i = 0;
    List.foldRight<Nat8, ()>(bytes, (), func (byte, _) {
      n += Prim.nat8ToNat(byte) * 256 ** i;
      i += 1;
    });
    n
  };

  public func natToBits(n : Nat) : List<Bool> {
    var a = 0;
    var b = n;
    var bits = List.nil<Bool>();
    var test = true;
    while test {
      a := b % 2;
      b := b / 2;
      bits := List.push<Bool>(a == 1, bits);
      test := b > 0;
    };
    bits
  };

  public func natFromBits(bits : List<Bool>) : Nat {
    var n = 0;
    var i = 0;
    List.foldRight<Bool, ()>(bits, (), func (test, _) {
      if test { n += 2 ** i };
      i += 1;
    });
    n
  };

}
