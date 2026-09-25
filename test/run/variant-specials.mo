func specials(one : { #one : Nat }, two : { #c0; #c1 }) {
  var count = 0;
  switch one {
    case #one(i) { count += i }
  };

  switch two {
    case #c0 { count += 1 }
    case #c1 { count += 2 }
  };

  count += 123;
};

specials(#one 42, #c1)

//CHECK: func $specials
//CHECK-NOT: {{i32|i64}}.const 5544550
//CHECK: local.set $count

//CHECK: {{i32|i64}}.const 22125
//CHECK: local.set $count

//CHECK-NOT: {{i32|i64}}.const 22126
//CHECK: local.set $count

//CHECK: {{i32.const 246|i64.const 494}}
