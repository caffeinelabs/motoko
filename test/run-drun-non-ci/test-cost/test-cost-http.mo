import Prim "mo:⛔";

actor client {
  func print(t : Text) = Prim.debugPrint("client: " # t);

  public type http_header = { value : Text; name : Text };
  public type http_response = {
    status : Nat;
    body : [Nat8];
    headers : [http_header];
  };
  public type http_request = {
    url : Text;
    method : { #get; #head; #post };
    max_response_bytes : ?Nat64;
    body : ?[Nat8];
    transform : ?{
      function : shared query {
        context : [Nat8];
        response : http_response;
      } -> async http_response;
      context : [Nat8];
    };
    headers : [http_header];
  };
  let ic00 = actor "aaaaa-aa" : actor {
    http_request : shared http_request -> async http_response;
  };

  // Local replica output:
  // already topped up; balance = 2_958_813_055_523
  // test request: {body = null; headers = []; max_response_bytes = null; method = #get; url = "https://ic0.app"}
  // 15 -- calculated request size
  // 20_849_218_000 -- http cost request
  // Cycles.balance()   = 2_916_705_501_695
  // Cycles.available() = 0
  // Cycles.refunded()  = 0
  // response: {body = []; headers = [{name = "location"; value = "https://dashboard.internetcomputer.org/"}, {name = "strict-transport-security"; value = "max-age=31536000; includeSubDomains"}, {name = "x-request-id"; value = "01964982-51da-77a0-8904-8ee869ccc1ec"}, {name = "content-length"; value = "0"}, {name = "date"; value = "Fri, 18 Apr 2025 15:26:38 GMT"}]; status = 307}
  // Cycles.balance()   = 2_895_850_399_717
  // Cycles.available() = 0
  // Cycles.refunded()  = 0
  // 20_855_101_978 -- Cycles.balance() diff
  public func go() : async () {
    let request : http_request = {
      url = "https://ic0.app";
      method = #get;
      headers = [];
      body = null;
      transform = null;
      max_response_bytes = null;
    };

    let headers = [{ name = "x-test"; value = "test" }];
    let body = ?[1, 2, 3] : ?[Nat8];
    let max_response_bytes = ?1_000 : ?Nat64;
    let transform = ?{
      function = transformFunction;
      context = [23, 41, 13, 6, 17] : [Nat8];
    };

    await test(request, true);
    await test({ request with headers }, true);
    await test({ request with body }, true);
    await test({ request with max_response_bytes }, true);
    await test({ request with headers; body; max_response_bytes }, true);

    // Not precise cases: For now there is no way to assess the exact cost of the transform function
    await test({ request with transform }, false);
    await test({ request with headers; body; max_response_bytes; transform }, false);
  };

  public shared query func transformFunction({
    context : [Nat8];
    response : http_response;
  }) : async http_response {
    ignore context;
    { response with body = []; headers = []; status = 200 };
  };

  func printCycles() {
    print("Cycles.balance()   = " # debug_show (Prim.cyclesBalance()));
    print("Cycles.available() = " # debug_show (Prim.cyclesAvailable()));
    print("Cycles.refunded()  = " # debug_show (Prim.cyclesRefunded()));
  };

  func test(request : http_request, isPrecise : Bool) : async () {
    let transformToPrint = switch (request.transform) {
      case (?{ function; context }) ?{
        function = to_candid (function);
        context;
      };
      case null null;
    };
    print("test request:\n" # debug_show ({ url = request.url; method = request.method; headers = request.headers; body = request.body; max_response_bytes = request.max_response_bytes; transform = transformToPrint }));

    let requestSize : Nat64 = calculateRequestSize(request);
    print(debug_show (requestSize) # " -- calculated request size");
    let defaultMaxResBytes : Nat64 = request.max_response_bytes ?? 2_000_000;
    let cost = Prim.costHttpRequest(requestSize, defaultMaxResBytes);
    print(debug_show (cost) # " -- http cost request");

    printCycles();
    let response = await (with cycles = cost) ic00.http_request(request); // This call should succeed, it should be enough cycles
    print("response: " # debug_show (response));
    printCycles();

    // Try the same request with less cycles, it should fail
    if (isPrecise) {
      try {
        let _ = await (with cycles = cost - 1) ic00.http_request(request);
        assert false; // Should not happen
      } catch (e) {
        print("error code: " # debug_show (Prim.errorCode(e)));
        print("error message: " # debug_show (Prim.errorMessage(e)));
      };
    };
    print("---");
  };

  /// [Source](https://github.com/dfinity/cdk-rs/pull/570/files#diff-c415c9ec7f29503dcb76e31fcac3062b0311cc0705f9159c1dfe3cd3308957b7R425)
  func calculateRequestSize(request : http_request) : Nat64 {
    var size : Nat64 = 0;

    // Add URL byte length
    size += Prim.natToNat64(request.url.size());

    // Add headers byte length (sum of all names and values)
    for (header in request.headers.vals()) {
      size += Prim.natToNat64(header.name.size());
      size += Prim.natToNat64(header.value.size());
    };

    // Add body length if present
    switch (request.body) {
      case (?body) { size += Prim.natToNat64(body.size()) };
      case null {};
    };

    // Add transform context length if present
    switch (request.transform) {
      case (?transform) {
        size += Prim.natToNat64(transform.context.size());
        // How to get the method name length otherwise?
        // This gets us both the method name and the actor
        let blob = to_candid (transform.function);
        size += Prim.natToNat64(blob.size());
      };
      case null {};
    };

    size;
  };
};

client.go(); //OR-CALL ingress go "DIDL\x00\x00"

//SKIP run
//SKIP run-ir
//SKIP run-low
