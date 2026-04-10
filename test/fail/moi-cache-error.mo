//MOC-FLAG --moi-cache _out/moi-cache-dir
import C "moi-cache-error/counter";

let c = C.make();
C.inc(c);

let x : Text = C.get(c); // type error: Nat vs Text
