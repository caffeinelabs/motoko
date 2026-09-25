//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY

// EOP backs every machine word with i64. The wrapping-pow body
// (`**%` / `**`) LSB-tests the exponent with
// `i64.shl 63; i64.eqz; if t e`, which the peephole collapses to
// `i64.ctz; i32.wrap_i64; if t e`. Same body is used for Nat64 `**`
// and for the TaggedSmallWord `**%` path.
let n : Nat64 = 3;
let exp : Nat64 = 10;
assert (n ** exp == 59049);

// CHECK-LABEL: (func $wpow_nat<Nat64>
// CHECK: i64.ctz
// CHECK-NEXT: i32.wrap_i64
// CHECK-NEXT: if  ;;
// CHECK: end)
