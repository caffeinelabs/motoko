import Prim "mo:⛔";

type MonoList = { #nil; #cons : MonoList };

func show_MonoList(x : MonoList) : Text = debug_show x;

Prim.debugPrint(show_MonoList(#cons (#cons (#nil))));
