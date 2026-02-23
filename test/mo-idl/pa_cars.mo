//MOC-FLAG -A=M0194
import T "import/car";

actor class Car(dmv : T.DMV) {
  /* Higher order function is supported for now.
  public func verifyCarInformation(user : User, car : T.Car) : async ?(shared (Location, TimeSpan) -> async Result) {
    let carInfo = await dmv.check(car);
    if (carInfo.isValid and not carInfo.wasStolen) {
      return ?(shared func (location:Location, time:TimeSpan) : async Result {
        return reserveSpot(user, carInfo, location, time);
      })
    } else {
      return null;
    }
  };
  */
  public func reserveSpot(user : User, carInfo : T.CarInfo, location : Location, timeSpan : TimeSpan) : async Result {
    // Do the actual work of registering the parking spot for the
    // given car in the given time span
    return null;
  };

  public type Location = { lat : Float; long : Float };
  public type TimeSpan = { start : Int; end : Int };
  public type Result = ?({ reservationId : Text });
  public type User = { name : Text };
};
