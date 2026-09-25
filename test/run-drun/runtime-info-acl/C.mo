actor class C(
  internal : actor {
    __motoko_runtime_information : () -> async {};
  }
) {

  // attempt to call runtime info for the actor passed as reference
  public shared func callback() : async () {
    ignore await internal.__motoko_runtime_information(); // should fail unless controller or selfaxs
  };
};
