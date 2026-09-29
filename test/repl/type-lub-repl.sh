#!/usr/bin/env bash
# Tests that correct lub types are inferred when values appear in arrays
moc -W M0074 -i lib/type-lub.mo <<__END__
opts;
nulls;
incompatible_objs;
objs;
tups;
tup1s;
arrs;
incompatible_funcs;
funcs;
funcs[0]([1, 2, 3]);
funcs[1]([1, 2, 3]);
poly_funcs;
poly_funcs[0]<Int>([1, 2, 3]);
poly_funcs[1]<Char>([1, 2, 3]);
poly_funcs2;
poly_funcs2[0]<Int>([1, 2, 3]);
poly_funcs2[1]<Char>([1, 2, 3]);
poly_funcs3;

variant_funcs;
variant_funcs[0](#bar);
variant_funcs[1](#bar);

mut_arrs;

shareds;
shared2s;
shared_funcs;
shared_funcs[0](25);
shared_funcs[1](25);

c0;
c1s;
__END__
