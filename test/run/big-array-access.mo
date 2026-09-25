let array = [1, 2, 3];

array[10000000000000];

// wasm-run output includes a backtrace printed by wasmrun, which changes
// depending on RTS compile flags
//SKIP wasm-run
