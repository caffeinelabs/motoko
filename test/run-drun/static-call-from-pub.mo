actor {
    flexible func init() {
      assert false;
    };
    public func reset() : () {
        init()
    };
};

// annoying administrative regexes introduced by our passes

// CHECK-LABEL: (func $reset (type
// CHECK-NOT: call_indirect
// CHECK: call $$lambda

// CHECK-LABEL: (func $$lambda
// CHECK-NOT: call_indirect
// CHECK: call $init
