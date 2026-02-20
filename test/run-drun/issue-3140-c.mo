//MOC-FLAG -A=M0194
import Prim = "mo:⛔";
actor {

  type None = Prim.Types.None;
  type Null = Prim.Types.Null;
  type Bool = Prim.Types.Bool;
  type Nat = Prim.Types.Nat;
  type Nat8 = Prim.Types.Nat8;
  type Nat16 = Prim.Types.Nat16;
  type Nat32 = Prim.Types.Nat32;
  type Nat64 = Prim.Types.Nat64;
  type Int = Prim.Types.Int;
  type Int8 = Prim.Types.Int8;
  type Int16 = Prim.Types.Int16;
  type Int32 = Prim.Types.Int32;
  type Int64 = Prim.Types.Int64;
  type Float = Prim.Types.Float;
  type Char = Prim.Types.Char;
  type Text = Prim.Types.Text;
  type Blob = Prim.Types.Blob;
  type Error = Prim.Types.Error;
  type Principal = Prim.Types.Principal;

  type Fresh = ?Fresh;

  type T = (
    None,
    Null,
    Bool,
    Nat,
    Nat8,
    Nat16,
    Nat32,
    Nat64,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Float,
    Char,
    Text,
    Blob,
    //Error,
    Principal,
    Fresh
  );

  stable var t = [] : [T];
}
