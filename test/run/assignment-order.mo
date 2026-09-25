// assignment order checks

var o = "";

// Array
([var 1, 2, do { o #= "a"; 3 }])[do { o #= "b"; 1 }] := do { o #= "c"; 42 };

assert o == "abc";

([var 1, 2, do { o #= "d"; 3 }])[do { o #= "e"; 2 }] += do { o #= "f"; 42 };

assert o == "abcdef";

// Object/record
{ a = 42; var b = do { o #= "g"; 25 } }.b := do { o #= "h"; 42 };

assert o == "abcdefgh";

{ a = 42; var b = do { o #= "i"; 25 } }.b += do { o #= "j"; 42 };

assert o == "abcdefghij";
