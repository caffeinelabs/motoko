// test parsing dot notation on integral literals works
module Nat {

  public func toText(self : Nat) : Text { "" };

};

module Int {

  public func toText(self : Int) : Text { "" };

};

module Float {

  public func toText(self : Float) : Text { "" };

};

ignore 0.toText(); // Nat.toText()

ignore 0 .toText(); // Nat.toText()

ignore (+0).toText(); // Int.toText()

ignore (-0).toText(); // Int.toText()

ignore 0.0.toText(); // Float.toText()

ignore 0..toText(); // Float.toText()

ignore +0 .toText(); // dubious (error)
ignore -0 .toText(); // dubious (error)
