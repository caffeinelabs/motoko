let _ = [1, 2, 3][1_180_591_620_717_411_303_424]

// The backtrace runs through the RTS, whose frames depend on how it was built
//FILTER wasm-run head -n 1
