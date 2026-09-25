type A = { #A };
type B = { #B };

func f2<T1, T2>(x1 : T1, x2 : T2) : (T1, T2) { (x1, x2) };

do {
  let (#A, #B) = #B |> f2<A, B>(_, #B); // type error
};

do {
  let (#A, #B) = #A |> f2<A, B>(#A, _); // type error
};

do {
  let (#A, #B) = #A |> f2<A, B> _; // type error
};
