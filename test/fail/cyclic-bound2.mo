/* check cyclic bounds are rejected, would loop sans check */

func f<A <: B, B <: A>(a : A, b : B) {
  let _ : () = a;
  let _ : () = b;
};
