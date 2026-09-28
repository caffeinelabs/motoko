import { debugPrint; errorMessage; call_raw; principalOfActor; charToText; performanceCounter } "mo:⛔";

// Space bomb tests

// Messages in this test all take a lot of time, memory and stack space to decode.
// With infinite resources, these are all valid Candid messages.
// When using Candid in a resource limited environment, for example one consensus round in a blockchain,
// an implementation with self-metering should reject these messages relatively early
// without going through the whole deserialisation process.

// \80\94\eb\dc\03  is 1000_000_000
// \80\ad\e2\04     is   10_000_000
// \ff\ff\3f        is    1_048_575
// \80\b5\18        is      400_000

// Tests manually ported from Candid test suite
// https://github.com/dfinity/candid/blob/master/test/spacebomb.test.did

// Currently we cannot run this particular Candid test suite
// because we rely on IC performance_counter(0) to limit execution.
// and the test suite is run on wasmtime (sans perf counter).

actor this {

  func toHex(b : Blob) : Text {
    let s = debug_show b;
    var t = "";
    for c in s.chars() {
      if not (c == '\"' or c == '\\') {
        t #= charToText(c)
      }
    };
    t
  };

  func assert_low_cost() {
    let limit : Nat64 = 50_000;
    let c = performanceCounter(0);
    if c > limit { debugPrint (debug_show c) };
    assert performanceCounter(0) < limit;
    debugPrint("decoded at low cost");
  };

  public func vec_null_extra_argument() : async () {
    assert_low_cost();
  };

  public func vec_reserved_extra_argument() : async () {
    assert_low_cost();
  };

  public func zero_sized_record_extra_argument() : async () {
    assert_low_cost();
  };

  public func vec_vec_null_extra_argument() : async () {
    assert_low_cost();
  };

  public func vec_record_emp_extra_argument() : async () {
    assert_low_cost();
  };

  public func vec_opt_record_with_2_20_null_extra_argument() : async () {
    assert_low_cost();
  };

  public func vec_null_not_ignored(_ : [?Nat]) : async () {
    assert_low_cost();
  };

  public func vec_reserved_not_ignored(_ : [Any]) : async () {
    assert_low_cost();
  };

  // this test may be broken
  public func zero_sized_record_not_ignored(_ : [{ _0_: Null; _1_: { _0_:Any }; _2_: {} }]) : async () {
    assert_low_cost();
  };

  public func vec_vec_null_not_ignored(_ : [[Null]]) : async () {
    assert_low_cost();
  };

  // this test may be broken
  public func vec_record_emp_not_ignored(_ : [{}]) : async () {
    assert_low_cost();
  };

  public func vec_null_subtyping(_ : ?Nat) : async () {
    assert_low_cost();
  };

  public func vec_reserved_subtyping(_ : ?Nat) : async () {
    assert_low_cost();
  };

  public func zero_sized_record_subtyping(_ : ?Nat) : async () {
    assert_low_cost();
  };

  public func vec_vec_null_subtyping(_ : [?Nat]) : async () {
    assert_low_cost();
  };

  public func vec_record_emp_subtyping(_ : ?Nat) : async () {
    assert_low_cost();
  };

  public func vec_opt_record_with_2_20_null_subtyping(_ : [?{}]) : async () {
    assert_low_cost();
  };

  func test(m : Text, blobs : [Blob]) : async* () {
    let p = principalOfActor(this);
    for blob in blobs.values() {
      debugPrint (debug_show { function = m; hex = toHex blob });
      try {
        ignore await call_raw(p, m, blob);
      }
      catch e {
        debugPrint(errorMessage(e));
      }
    }
  };

  public func go() : async () {

    // Plain decoding (unused arguments)
    await* test("vec_null_extra_argument", [
      "DIDL\01\6d\7f\01\00\80\94\eb\dc\03"
    ]);

    await* test("vec_reserved_extra_argument", [
      "DIDL\01\6d\70\01\00\80\94\eb\dc\03"
    ]);

    await* test("zero_sized_record_extra_argument", [
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\94\eb\dc\03"
    ]);

    await* test("vec_vec_null_extra_argument", [
      "DIDL\02\6d\01\6d\7f\01\00\05\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f"
    ]);

    await* test("vec_record_emp_extra_argument", [
      "DIDL\02\6d\01\6c\00\01\00\80\ad\e2\04"
    ]);

    await* test("vec_opt_record_with_2_20_null_extra_argument", [
      "DIDL\17\6c\02\01\7f\02\7f\6c\02\01\00\02\00\6c\02\00\01\01\01\6c\02\00\02\01\02\6c\02\00\03\01\03\6c\02\00\04\01\04\6c\02\00\05\01\05\6c\02\00\06\01\06\6c\02\00\07\01\07\6c\02\00\08\01\08\6c\02\00\09\01\09\6c\02\00\0a\01\0a\6c\02\00\0b\01\0b\6c\02\00\0c\01\0c\6c\02\00\0d\02\0d\6c\02\00\0e\01\0e\6c\02\00\0f\01\0f\6c\02\00\10\01\10\6c\02\00\11\01\11\6c\02\00\12\01\12\6c\02\00\13\01\13\6e\14\6d\15\01\16\05\01\01\01\01\01"
    ]);

    // Decoding to actual type
    await* test("vec_null_not_ignored", [
      "DIDL\01\6d\7f\01\00\80\94\eb\dc\03",
      "DIDL\01\6d\7f\01\00\80\ad\e2\04",
      "DIDL\01\6d\7f\01\00\ff\ff\3f",
      "DIDL\01\6d\7f\01\00\80\bf\18"
    ]);

    await* test("vec_reserved_not_ignored", [
      "DIDL\01\6d\70\01\00\80\94\eb\dc\03" : Blob,
      "DIDL\01\6d\70\01\00\80\ad\e2\04" : Blob,
      "DIDL\01\6d\70\01\00\ff\ff\3f" : Blob,
      "DIDL\01\6d\70\01\00\80\bf\18" : Blob,
    ]);

    await* test("zero_sized_record_not_ignored", [
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\94\eb\dc\03",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\ad\e2\04",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\ff\ff\3f",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\b5\18",
    ]);

    await* test("vec_vec_null_not_ignored", [
      "DIDL\02\6d\01\6d\7f\01\00\05\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f"
    ]);

    await* test("vec_record_emp_not_ignored", [
      "DIDL\02\6d\01\6c\00\01\00\80\ad\e2\04"
    ]);

    // Decoding under opt
    await* test("vec_null_subtyping", [
      "DIDL\01\6d\7f\01\00\80\94\eb\dc\03" : Blob,
      "DIDL\01\6d\7f\01\00\80\ad\e2\04" : Blob,
      "DIDL\01\6d\7f\01\00\ff\ff\3f" : Blob,
      "DIDL\01\6d\7f\01\00\80\bf\18" : Blob,
    ]);

    await* test("vec_reserved_subtyping", [
      "DIDL\01\6d\70\01\00\80\94\eb\dc\03" : Blob,
      "DIDL\01\6d\70\01\00\80\ad\e2\04" : Blob,
      "DIDL\01\6d\70\01\00\ff\ff\3f" : Blob,
      "DIDL\01\6d\70\01\00\80\bf\18" : Blob,
    ]);

    await* test("zero_sized_record_subtyping", [
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\94\eb\dc\03",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\ad\e2\04",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\ff\ff\3f",
      "DIDL\04\6c\03\00\7f\01\01\02\02\6c\01\00\70\6c\00\6d\00\01\03\80\b5\18",
    ]);

    await* test("vec_vec_null_subtyping", [
      "DIDL\02\6d\01\6d\7f\01\00\05\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f\ff\ff\3f"
    ]);

    await* test("vec_record_emp_subtyping", [
      "DIDL\02\6d\01\6c\00\01\00\80\ad\e2\04"
    ]);

    await* test("vec_opt_record_with_2_20_null_subtyping", [
      "DIDL\17\6c\02\01\7f\02\7f\6c\02\01\00\02\00\6c\02\00\01\01\01\6c\02\00\02\01\02\6c\02\00\03\01\03\6c\02\00\04\01\04\6c\02\00\05\01\05\6c\02\00\06\01\06\6c\02\00\07\01\07\6c\02\00\08\01\08\6c\02\00\09\01\09\6c\02\00\0a\01\0a\6c\02\00\0b\01\0b\6c\02\00\0c\01\0c\6c\02\00\0d\02\0d\6c\02\00\0e\01\0e\6c\02\00\0f\01\0f\6c\02\00\10\01\10\6c\02\00\11\01\11\6c\02\00\12\01\12\6c\02\00\13\01\13\6e\14\6d\15\01\16\05\01\01\01\01\01"
    ]);

  }

}
//CALL ingress go "DIDL\x00\x00"
//SKIP run
//SKIP run-ir
//SKIP run-low
