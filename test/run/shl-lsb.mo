//CLASSICAL-PERSISTENCE-ONLY

// In classical mode, the wrapping-pow body (`**%`) LSB-tests the
// exponent with `i32.shl 31; eqz; if t e` — the peephole collapses
// this to `i32.ctz; if t e` (no leg swap; eqz consumed inverted the
// test). Both `Nat32 **%` and `Int32 **%` delegate to the shared
// `wpow_nat<Nat32>` body, so one CHECK suffices.
let n : Nat32 = 3;
let exp : Nat32 = 5;
assert (n **% exp == 243);

let i : Int32 = -3;
let iexp : Int32 = 5;
assert (i **% iexp == -243);

// CHECK-LABEL: (func $wpow_nat<Nat32>
// CHECK: i32.ctz
// CHECK-NEXT: if  ;;
// CHECK: end)
