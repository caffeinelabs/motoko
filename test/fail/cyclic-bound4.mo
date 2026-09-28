/* check cyclic bounds are rejected, would loop sans check */

type C<U, V> = U;

type T<U> = C<U,U>;

func f<A <: T<B>, B <: T<C>, C <: T<A>>(a : A, b : B, c : C) {
  let _ : () = a;
  let _ : () = b;
  let _ : () = c;
};
