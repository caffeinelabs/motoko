import Debug "mo:core/Debug";

persistent actor Alarm {

  let n = 5;
  var count = 0;

  public shared func ring() : async () {
    Debug.print("Ring!");
  };

  system func heartbeat() : async () {
    if (count % n == 0) {
      await ring();
    };
    count += 1;
  }
}
