import Runtime "mo:core/Runtime";
import Float "mo:core/Float";
import Time "mo:core/Time";
import {migration} "Migration";

(with migration) // use the imported migration function
persistent actor
  Counter_v9 {

  var state : Float = 0.0; // expicitly migrated

  var lastModified : Time.Time = Time.now(); // implicitly migrated

  public func increment() : async () {
    lastModified := Time.now();
    state += 0.5;
  };

  public func decrement() : async () {
    lastModified := Time.now();
    state -= 0.5;
  };

  public query func read() : async Int {
    Runtime.trap("No longer supported: Use `readFloat`");
  };

  public query func readFloat() : async Float {
    return state;
  };

  public query func lastAccess() : async Time.Time {
    return lastModified;
  };

};
