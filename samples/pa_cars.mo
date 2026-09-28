// Stateless Suspendable Workflow
// ==============================

// Creating an Account for the PA parking service

// - Client: Insert Car data and holder's name
// - Server: Validate car is registered under the given name
// - Client: Pick a parking spot from a Google Map like interface + time
// - Server: Register the parking spot for the user
type Car = { model : Text; plate : Text };
type DMV = actor { check : Car -> async CarInfo };
type CarInfo = {
  model : Text;
  plate : Text;
  isValid : Bool;
  wasStolen : Bool;
  expires : Nat;
};

persistent actor class PACars(dmv : DMV) {
  public func verifyCarInformation(user : User, car : Car) : async ?(shared (Location, TimeSpan) -> async Result) {
    let carInfo = await dmv.check(car);
    if carInfo.isValid and not carInfo.wasStolen {
      return ?(shared func (location : Location, time : TimeSpan) : async Result {
        return reserveSpot(user, carInfo, location, time);
      })
    } else {
      return null;
    }
  };

  private func reserveSpot(user : User, carInfo : CarInfo, location : Location, timeSpan : TimeSpan) : Result {
    // Do the actual work of registering the parking spot for the
    // given car in the given time span
    return null;
  };

  public type Location = { lat : Float; long : Float };
  public type TimeSpan = { start : Int; end : Int };
  public type Result = ?({ reservationId : Text });
  public type User = { name : Text };
};
