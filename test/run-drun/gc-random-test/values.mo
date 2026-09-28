import Array "array";
import Region "../stable-region/Region";

module {
  public type RandomValue = {
    #none;
    #obj : RandomObject;
    #arr : RandomArray;
    #blob : RandomBlob;
    #box : RandomBox;
    #text : RandomText;
    #option : RandomOption;
    #region : RandomRegion;
  };

  public type RandomObject = {
    var randomNumber : Nat;
    var ref1 : RandomValue;
    var ref2 : RandomValue;
    checkNumber : Nat;
  };

  public type RandomBox = {
    var randomBigInt : Int;
    checkBigInt : Int;
    var randomFloat : Float;
    checkFloat : Float;
    var randomInt64 : Int64;
    checkInt64 : Int64;
  };

  public type RandomText = {
    var randomText : Text;
    checkText : Text;
  };

  public type RandomArray = [var RandomValue];

  public type RandomBlob = Blob;

  public type RandomOption = ?RandomValue;

  public type RandomRegion = {
    region : Region;
    checkSize : Nat64;
    checkValue : Nat8;
  };

  public func isNone(value : RandomValue) : Bool {
    switch value {
      case #none { true }
      case _ { false }
    };
  };

  public func filterReferences(array : [RandomValue]) : [RandomValue] {
    Array.filter<RandomValue>(array, func(element) { not isNone(element) });
  };
};
