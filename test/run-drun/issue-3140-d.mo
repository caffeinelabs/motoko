//MOC-FLAG -A=M0194
actor {

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
