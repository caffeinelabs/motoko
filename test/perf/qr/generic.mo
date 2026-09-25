/**
 * Module     : generic.mo
 * Copyright  : 2020 DFINITY Stiftung
 * License    : Apache 2.0 with LLVM Exception
 * Stability  : Stable
 */

import Alphanumeric "alphanumeric";
import Common "common";
import EightBit "eight-bit";
import Kanji "kanji";
import List "list";
import Numeric "numeric";
import Version "version";

module {

  type List<T> = List.List<T>;
  type Mode = Common.Mode;
  type Version = Version.Version;

  public func encode(
    version : Version,
    mode : Mode,
    text : Text
  ) : ?List<Bool> {
    switch mode {
      case #Alphanumeric { Alphanumeric.encode(version, text) }
      case #EightBit { EightBit.encode(version, text) }
      case #Kanji { Kanji.encode(version, text) }
      case #Numeric { Numeric.encode(version, text) }
    }
  };

}
