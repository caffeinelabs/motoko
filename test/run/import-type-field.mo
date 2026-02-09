import { type List; cons } = "lib/ListM";
let { type C } : module { type A = Int; type B = Int; type C = Int } =
  module { public type A = Int; public type B = Int; public type C = Int; public type D = Int };
