//MOC-FLAG -A=M0194
/*
This is a snapshot of https://github.com/enzoh/qr
at revision 3d4537cba3418f28a0b3ed53fe4485bbce3d10f8
with the following changes:
 * copied all files to qr/
 * copied relevant files from stdlib to qr/
 * updates imports accordingly
 * added a go() method to the canister with some test data

*/

/**
 * Module     : qr.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : Stable
 */

import Array "./qr/array";
import Block "./qr/block";
import Common "./qr/common";
import Generic "./qr/generic";
import List "./qr/list";
import Mask "./qr/mask";
import Option "./qr/option";
import Symbol "./qr/symbol";
import Version "./qr/version";
import Prelude "./qr/prelude";
import Prim "mo:⛔";

actor QR {

  type List<T> = List.List<T>;

  public type ErrorCorrection = Common.ErrorCorrection;
  public type Matrix = Common.Matrix;
  public type Mode = Common.Mode;
  public type Version = Version.Version;

  public func encode(
    version : Version,
    level : ErrorCorrection,
    mode : Mode,
    text : Text,
  ) : async ?Matrix {
    Option.bind<Version, Matrix>(
      Version.new(Version.unbox(version)),
      func _ {
        Option.bind<List<Bool>, Matrix>(
          Generic.encode(version, mode, text),
          func(data) {
            Option.bind<List<Bool>, Matrix>(
              Block.interleave(version, level, data),
              func(code) {
                let (arrays, maskRef) = Mask.generate(version, level, code);
                ?#Matrix(
                  Symbol.freeze(
                    Symbol.applyVersions(
                      version,
                      Symbol.applyFormats(version, level, maskRef, arrays),
                    )
                  )
                );
              },
            );
          },
        );
      },
    );
  };

  public func show(matrix : Matrix) : async Text {
    let #Matrix arrays = matrix;
    Array.foldl<[Bool], Text>(
      func(accum1, array) {
        Array.foldl<Bool, Text>(
          func(accum2, bit) {
            let text = if bit "##" else "  ";
            text # accum2;
          },
          "\n",
          array,
        ) # accum1;
      },
      "",
      arrays,
    );
  };

  public func go() : async () {
    let tests = [
      (#Version 1, #M, #Numeric, "01234567"),
      (#Version 1, #Q, #Alphanumeric, "HELLO WORLD"),
      (#Version 2, #M, #Alphanumeric, "HTTPS://SDK.DFINITY.ORG"),
    ];
    for ((version, level, mode, text) in tests.values()) {
      let result = await QR.encode(version, level, mode, text);
      Prelude.printLn(
        switch result {
          case (?matrix) "\n" # (await QR.show(matrix));
          case _ "Error: Invalid input!";
        }
      );
    };
  };

  public func getPerfData() : async () {
    Prim.debugPrint("instructions: " # debug_show (Prim.rts_lifetime_instructions()));
  };
};

//CALL ingress go 0x4449444C0000
//CALL ingress getPerfData 0x4449444C0000
