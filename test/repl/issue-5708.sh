#!/usr/bin/env bash
# Tests redeclaration of types
moc -i <<__END__
type t = <T> T -> <T> T -> T;
type T = ();
type t = <T> T -> <T> T -> T;
__END__
