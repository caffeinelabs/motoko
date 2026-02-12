//MOC-FLAG --package core ../core-stub/src

import Time "mo:core/Time";

let t : Time.Time = Time.now();
assert (t.unwrap == 0);

let t2 : Time.Time = Time.Time(42);
assert (t2.unwrap == 42);
