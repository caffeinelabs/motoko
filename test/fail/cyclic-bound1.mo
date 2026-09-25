/* check cyclic bounds are rejected, would loop sans check */

func f<A <: A>(a : A)
{
  let _ : () = a;
};
