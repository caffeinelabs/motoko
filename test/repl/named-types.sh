#!/usr/bin/env bash
# Tests inference of named types
moc -W M0145 -i <<__END__
func (#tag (f:Nat)){};
func (#tag (_:Nat)){};
func ({x : Nat}){};
func ({x = i : Nat}) {};
func ((?x):?Nat) {};
func (x : Nat) {};
func (x : Nat, y : Nat, z : Nat) {};
func (?(x : Nat, y : Nat, z : Nat)) {};
func (?(x : Nat)) {};
__END__
