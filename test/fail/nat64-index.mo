// Int types as array index should be rejected
let a = [1, 2, 3];
let i8 : Int8 = 0;
ignore a[i8];

let i16 : Int16 = 0;
ignore a[i16];

let i32 : Int32 = 0;
ignore a[i32];

let i64 : Int64 = 0;
ignore a[i64];

// Int as array index should be rejected
let i : Int = 0;
ignore a[i];

// Int types as blob index should be rejected
let b : Blob = "hello";
ignore b[i8];
ignore b[i16];
ignore b[i32];
ignore b[i64];
ignore b[i];
