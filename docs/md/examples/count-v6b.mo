import Runtime "mo:core/Runtime";
import Float "mo:core/Float";

(with migration =
  func (_ : {var state : Int}) : {} { // discard old state
   {}
  }
)
persistent actor Counter_v6 {
  var newState : Float = 0.0; // implicitly `stable`

  public func increment() : async () {
    newState += 0.5;
  };

  public func decrement() : async () {
    newState -= 0.5;
  };

  public query func read() : async Int {
    Runtime.trap("No longer supported: Use `readFloat`");
  };

  public query func readFloat() : async Float {
    return newState;
  };
};
