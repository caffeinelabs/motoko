// A record literal written where braces mean a block; the fix is nesting it as the block's result (M0269).
func f() : { x : Nat } {
  x = 0
};
