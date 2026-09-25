/* check cyclic bounds are rejected, would loop sans check */

type T<U> = U;

type C<U <: T<U>> = ();
