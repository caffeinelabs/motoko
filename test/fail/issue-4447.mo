import { debugPrint; setTimer } = "mo:⛔";

func _f() {

  func indirect<system>() {
    ignore setTimer<system>(0, false, func () : async () { debugPrint "YEP!" });
  };

  indirect<system>();
  ignore indirect<system>()
};

class _C() {

  func indirect<system>() {
    ignore setTimer<system>(0, false, func () : async () { debugPrint "YEP!" });
  };

  indirect<system>();
  ignore indirect<system>()
}
