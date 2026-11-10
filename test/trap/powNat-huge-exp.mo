let _ = (1 : Nat) ** (0x1_0000_0000 : Nat)

// The backtrace runs through the RTS, whose frames depend on how it was built
//FILTER wasm-run head -n 1
