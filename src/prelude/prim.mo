/*
The primitive definitions.

This module should contain everything that cannot be implemented in plain
Motoko. It is available via `import Prim "mo:⛔"`. Normal user code would
usually not import that module directly, but through `base`, which takes
care of providing a proper module structure, e.g. exposing `Array_tabulate`
through `Array.tabulate`.

Therefore, the prim module does not need to provide a stable interface, as its
only supported consumer is the `base` library, and that is bundled with the
compiler.

Nevertheless, it shoud be _safe_ to import prim, i.e. the definitions here
should not break type safety or other guarantees of the language.
*/

module Types = {
  public type Any = prim "Any";
  public type None = prim "None";
  public type Null = prim "Null";
  public type Bool = prim "Bool";
  public type Nat = prim "Nat";
  public type Nat8 = prim "Nat8";
  public type Nat16 = prim "Nat16";
  public type Nat32 = prim "Nat32";
  public type Nat64 = prim "Nat64";
  public type Int = prim "Int";
  public type Int8 = prim "Int8";
  public type Int16 = prim "Int16";
  public type Int32 = prim "Int32";
  public type Int64 = prim "Int64";
  public type Float = prim "Float";
  public type Char = prim "Char";
  public type Text = prim "Text";
  public type Blob = prim "Blob";
  public type Error = prim "Error";
  public type Principal = prim "Principal";
  public type Region = prim "Region";
};

func abs(x : Int) : Nat { (prim "abs" : Int -> Nat) x };
func shiftLeft(x : Nat, shift : Nat32) : Nat {
  (prim "lsh_Nat" : (Nat, Nat32) -> Nat)(x, shift);
};
func shiftRight(x : Nat, shift : Nat32) : Nat {
  (prim "rsh_Nat" : (Nat, Nat32) -> Nat)(x, shift);
};

// for testing
func idlHash(x : Text) : Nat32 { (prim "idlHash" : Text -> Nat32) x };

// Printing

func debugPrint(x : Text) { (prim "print" : Text -> ()) x };
func debugPrintNat(x : Nat) { debugPrint(@text_of_Nat x) };
func debugPrintInt(x : Int) { debugPrint(@text_of_Int x) };
func debugPrintChar(x : Char) { debugPrint(charToText x) };

// Trapping

func trap(x : Text) : None { (prim "trap" : Text -> None) x };

// RTS stats

func rts_version() : Text { (prim "rts_version" : () -> Text)() };
func rts_memory_size() : Nat { (prim "rts_memory_size" : () -> Nat)() };
func rts_heap_size() : Nat { (prim "rts_heap_size" : () -> Nat)() };
func rts_total_allocation() : Nat {
  (prim "rts_total_allocation" : () -> Nat)();
};
func rts_reclaimed() : Nat { (prim "rts_reclaimed" : () -> Nat)() };
func rts_max_live_size() : Nat { (prim "rts_max_live_size" : () -> Nat)() };
func rts_max_stack_size() : Nat { (prim "rts_max_stack_size" : () -> Nat)() };
func rts_callback_table_count() : Nat {
  (prim "rts_callback_table_count" : () -> Nat)();
};
func rts_callback_table_size() : Nat {
  (prim "rts_callback_table_size" : () -> Nat)();
};
func rts_mutator_instructions() : Nat {
  (prim "rts_mutator_instructions" : () -> Nat)();
};
func rts_collector_instructions() : Nat {
  (prim "rts_collector_instructions" : () -> Nat)();
};
func rts_upgrade_instructions() : Nat {
  (prim "rts_upgrade_instructions" : () -> Nat)();
};

func rts_stable_memory_size() : Nat {
  (prim "rts_stable_memory_size" : () -> Nat) ()
};

func rts_logical_stable_memory_size() : Nat {
  (prim "rts_logical_stable_memory_size" : () -> Nat) ()
};

// Total conversions (fixed to big)

let int64ToInt = @int64ToInt;
let int32ToInt = @int32ToInt;
let int16ToInt = @int16ToInt;
let int8ToInt = @int8ToInt;
let nat64ToNat = @nat64ToNat;
let nat32ToNat = @nat32ToNat;
let nat16ToNat = @nat16ToNat;
let nat8ToNat = @nat8ToNat;

// Trapping conversions (big to fixed)

func intToInt64(n : Int) : Int64 = (prim "num_conv_Int_Int64" : Int -> Int64) n;
func intToInt32(n : Int) : Int32 = (prim "num_conv_Int_Int32" : Int -> Int32) n;
func intToInt16(n : Int) : Int16 = (prim "num_conv_Int_Int16" : Int -> Int16) n;
func intToInt8(n : Int) : Int8 = (prim "num_conv_Int_Int8" : Int -> Int8) n;

func natToNat64(n : Nat) : Nat64 = (prim "num_conv_Nat_Nat64" : Nat -> Nat64) n;
func natToNat32(n : Nat) : Nat32 = (prim "num_conv_Nat_Nat32" : Nat -> Nat32) n;
func natToNat16(n : Nat) : Nat16 = (prim "num_conv_Nat_Nat16" : Nat -> Nat16) n;
func natToNat8(n : Nat) : Nat8 = (prim "num_conv_Nat_Nat8" : Nat -> Nat8) n;

// Wrapping conversions (big to fixed, and within fixed)

func intToInt64Wrap(n : Int) : Int64 = (prim "num_wrap_Int_Int64" : Int -> Int64) n;
func intToInt32Wrap(n : Int) : Int32 = (prim "num_wrap_Int_Int32" : Int -> Int32) n;
func intToInt16Wrap(n : Int) : Int16 = (prim "num_wrap_Int_Int16" : Int -> Int16) n;
func intToInt8Wrap(n : Int) : Int8 = (prim "num_wrap_Int_Int8" : Int -> Int8) n;

func intToNat64Wrap(n : Int) : Nat64 = (prim "num_wrap_Int_Nat64" : Int -> Nat64) n;
func intToNat32Wrap(n : Int) : Nat32 = (prim "num_wrap_Int_Nat32" : Int -> Nat32) n;
func intToNat16Wrap(n : Int) : Nat16 = (prim "num_wrap_Int_Nat16" : Int -> Nat16) n;
func intToNat8Wrap(n : Int) : Nat8 = (prim "num_wrap_Int_Nat8" : Int -> Nat8) n;

// Across same width
func int64ToNat64(n : Int64) : Nat64 = (prim "num_wrap_Int64_Nat64" : Int64 -> Nat64) n;
func nat64ToInt64(n : Nat64) : Int64 = (prim "num_wrap_Nat64_Int64" : Nat64 -> Int64) n;
func int32ToNat32(n : Int32) : Nat32 = (prim "num_wrap_Int32_Nat32" : Int32 -> Nat32) n;
func nat32ToInt32(n : Nat32) : Int32 = (prim "num_wrap_Nat32_Int32" : Nat32 -> Int32) n;
func int16ToNat16(n : Int16) : Nat16 = (prim "num_wrap_Int16_Nat16" : Int16 -> Nat16) n;
func nat16ToInt16(n : Nat16) : Int16 = (prim "num_wrap_Nat16_Int16" : Nat16 -> Int16) n;
func int8ToNat8(n : Int8) : Nat8 = (prim "num_wrap_Int8_Nat8" : Int8 -> Nat8) n;
func nat8ToInt8(n : Nat8) : Int8 = (prim "num_wrap_Nat8_Int8" : Nat8 -> Int8) n;

// Between adjacent widths
func nat8ToNat16(n : Nat8) : Nat16 = (prim "num_conv_Nat8_Nat16" : Nat8 -> Nat16) n;
func nat16ToNat32(n : Nat16) : Nat32 = (prim "num_conv_Nat16_Nat32" : Nat16 -> Nat32) n;
func nat32ToNat64(n : Nat32) : Nat64 = (prim "num_conv_Nat32_Nat64" : Nat32 -> Nat64) n;

func nat64ToNat32(n : Nat64) : Nat32 = (prim "num_conv_Nat64_Nat32" : Nat64 -> Nat32) n;
func nat32ToNat16(n : Nat32) : Nat16 = (prim "num_conv_Nat32_Nat16" : Nat32 -> Nat16) n;
func nat16ToNat8(n : Nat16) : Nat8 = (prim "num_conv_Nat16_Nat8" : Nat16 -> Nat8) n;

func int8ToInt16(n : Int8) : Int16 = (prim "num_conv_Int8_Int16" : Int8 -> Int16) n;
func int16ToInt32(n : Int16) : Int32 = (prim "num_conv_Int16_Int32" : Int16 -> Int32) n;
func int32ToInt64(n : Int32) : Int64 = (prim "num_conv_Int32_Int64" : Int32 -> Int64) n;

func int64ToInt32(n : Int64) : Int32 = (prim "num_conv_Int64_Int32" : Int64 -> Int32) n;
func int32ToInt16(n : Int32) : Int16 = (prim "num_conv_Int32_Int16" : Int32 -> Int16) n;
func int16ToInt8(n : Int16) : Int8 = (prim "num_conv_Int16_Int8" : Int16 -> Int8) n;

// Char conversion and properties

func charToNat32(c : Char) : Nat32 = (prim "num_wrap_Char_Nat32" : Char -> Nat32) c;
func nat32ToChar(w : Nat32) : Char = (prim "num_conv_Nat32_Char" : Nat32 -> Char) w;

func charToText(c : Char) : Text = (prim "conv_Char_Text" : Char -> Text) c;

func charToUpper(c : Char) : Char = (prim "char_to_upper" : Char -> Char) c;
func charToLower(c : Char) : Char = (prim "char_to_lower" : Char -> Char) c;
func charIsWhitespace(c : Char) : Bool = (prim "char_is_whitespace" : Char -> Bool) c;
func charIsLowercase(c : Char) : Bool = (prim "char_is_lowercase" : Char -> Bool) c;
func charIsUppercase(c : Char) : Bool = (prim "char_is_uppercase" : Char -> Bool) c;
func charIsAlphabetic(c : Char) : Bool = (prim "char_is_alphabetic" : Char -> Bool) c;

// Blob functions

func blobCompare(b1 : Blob, b2 : Blob) : Int8 = (prim "blob_compare" : (Blob, Blob) -> Int8)(b1, b2);
func hashBlob(b : Blob) : Nat32 { (prim "crc32Hash" : Blob -> Nat32) b };

// Text conversion
func decodeUtf8(b : Blob) : ?Text = (prim "decodeUtf8" : Blob -> ?Text) b;
func encodeUtf8(t : Text) : Blob = (prim "encodeUtf8" : Text -> Blob) t;

// Text comparison
func textCompare(t1 : Text, t2 : Text) : Int8 = (prim "text_compare" : (Text, Text) -> Int8)(t1, t2);

// Text lowercase
func textLowercase(t : Text) : Text = (prim "text_lowercase" : (Text) -> Text) (t);
// Text uppercase
func textUppercase(t : Text) : Text = (prim "text_uppercase" : (Text) -> Text) (t);

// Exotic bitwise operations
func popcntNat8(w : Nat8) : Nat8 = (prim "popcnt8" : Nat8 -> Nat8) w;
func clzNat8(w : Nat8) : Nat8 = (prim "clz8" : Nat8 -> Nat8) w;
func ctzNat8(w : Nat8) : Nat8 = (prim "ctz8" : Nat8 -> Nat8) w;
func btstNat8(w : Nat8, amount : Nat8) : Bool = (prim "btst8" : (Nat8, Nat8) -> Nat8)(w, amount) != (0 : Nat8);

func popcntNat16(w : Nat16) : Nat16 = (prim "popcnt16" : Nat16 -> Nat16) w;
func clzNat16(w : Nat16) : Nat16 = (prim "clz16" : Nat16 -> Nat16) w;
func ctzNat16(w : Nat16) : Nat16 = (prim "ctz16" : Nat16 -> Nat16) w;
func btstNat16(w : Nat16, amount : Nat16) : Bool = (prim "btst16" : (Nat16, Nat16) -> Nat16)(w, amount) != (0 : Nat16);

func popcntNat32(w : Nat32) : Nat32 = (prim "popcnt32" : Nat32 -> Nat32) w;
func clzNat32(w : Nat32) : Nat32 = (prim "clz32" : Nat32 -> Nat32) w;
func ctzNat32(w : Nat32) : Nat32 = (prim "ctz32" : Nat32 -> Nat32) w;
func btstNat32(w : Nat32, amount : Nat32) : Bool = (prim "btst32" : (Nat32, Nat32) -> Nat32)(w, amount) != (0 : Nat32);

func popcntNat64(w : Nat64) : Nat64 = (prim "popcnt64" : Nat64 -> Nat64) w;
func clzNat64(w : Nat64) : Nat64 = (prim "clz64" : Nat64 -> Nat64) w;
func ctzNat64(w : Nat64) : Nat64 = (prim "ctz64" : Nat64 -> Nat64) w;
func btstNat64(w : Nat64, amount : Nat64) : Bool = (prim "btst64" : (Nat64, Nat64) -> Nat64)(w, amount) != (0 : Nat64);

func popcntInt8(w : Int8) : Int8 = (prim "popcntInt8" : Int8 -> Int8) w;
func clzInt8(w : Int8) : Int8 = (prim "clzInt8" : Int8 -> Int8) w;
func ctzInt8(w : Int8) : Int8 = (prim "ctzInt8" : Int8 -> Int8) w;
func btstInt8(w : Int8, amount : Int8) : Bool = (prim "btstInt8" : (Int8, Int8) -> Int8) (w, amount) != (0 : Int8);

func popcntInt16(w : Int16) : Int16 = (prim "popcntInt16" : Int16 -> Int16) w;
func clzInt16(w : Int16) : Int16 = (prim "clzInt16" : Int16 -> Int16) w;
func ctzInt16(w : Int16) : Int16 = (prim "ctzInt16" : Int16 -> Int16) w;
func btstInt16(w : Int16, amount : Int16) : Bool = (prim "btstInt16" : (Int16, Int16) -> Int16) (w, amount) != (0 : Int16);

func popcntInt32(w : Int32) : Int32 = (prim "popcntInt32" : Int32 -> Int32) w;
func clzInt32(w : Int32) : Int32 = (prim "clzInt32" : Int32 -> Int32) w;
func ctzInt32(w : Int32) : Int32 = (prim "ctzInt32" : Int32 -> Int32) w;
func btstInt32(w : Int32, amount : Int32) : Bool = (prim "btstInt32" : (Int32, Int32) -> Int32)(w, amount) != (0 : Int32);

func popcntInt64(w : Int64) : Int64 = (prim "popcntInt64" : Int64 -> Int64) w;
func clzInt64(w : Int64) : Int64 = (prim "clzInt64" : Int64 -> Int64) w;
func ctzInt64(w : Int64) : Int64 = (prim "ctzInt64" : Int64 -> Int64) w;
func btstInt64(w : Int64, amount : Int64) : Bool = (prim "btstInt64" : (Int64, Int64) -> Int64)(w, amount) != (0 : Int64);

// Float operations

func floatAbs(f : Float) : Float = (prim "fabs" : Float -> Float) f;
func floatSqrt(f : Float) : Float = (prim "fsqrt" : Float -> Float) f;
func floatCeil(f : Float) : Float = (prim "fceil" : Float -> Float) f;
func floatFloor(f : Float) : Float = (prim "ffloor" : Float -> Float) f;
func floatTrunc(f : Float) : Float = (prim "ftrunc" : Float -> Float) f;
func floatNearest(f : Float) : Float = (prim "fnearest" : Float -> Float) f;
func floatMin(f : Float, g : Float) : Float = (prim "fmin" : (Float, Float) -> Float)(f, g);
func floatMax(f : Float, g : Float) : Float = (prim "fmax" : (Float, Float) -> Float)(f, g);
func floatCopySign(f : Float, g : Float) : Float = (prim "fcopysign" : (Float, Float) -> Float)(f, g);
func floatToInt(f : Float) : Int = (prim "num_conv_Float_Int" : Float -> Int)(f);
func intToFloat(n : Int) : Float = (prim "num_conv_Int_Float" : Int -> Float)(n);
func floatToInt64(f : Float) : Int64 = (prim "num_conv_Float_Int64" : Float -> Int64)(f);
func int64ToFloat(n : Int64) : Float = (prim "num_conv_Int64_Float" : Int64 -> Float)(n);

let floatToText = @text_of_Float;

// Configurable Float formatter
// mode:
//  0) fixed format "%.*f"
//  1) exponent format "%.*e"
//  2) generic format "%.*g"
//  3) hexadecimal format "%.*h"
//  _) invalid (traps)
func floatToFormattedText(f : Float, prec : Nat8, mode : Nat8) : Text = (prim "fmtFloat->Text" : (Float, Nat8, Nat8) -> Text)(f, prec, mode);

// Trigonometric and transcendental functions

func sin(f : Float) : Float = (prim "fsin" : Float -> Float) f;
func cos(f : Float) : Float = (prim "fcos" : Float -> Float) f;
func tan(f : Float) : Float = (prim "ftan" : Float -> Float) f;
func arcsin(f : Float) : Float = (prim "fasin" : Float -> Float) f;
func arccos(f : Float) : Float = (prim "facos" : Float -> Float) f;
func arctan(f : Float) : Float = (prim "fatan" : Float -> Float) f;
func arctan2(y : Float, x : Float) : Float = (prim "fatan2" : (Float, Float) -> Float)(y, x);

func exp(f : Float) : Float = (prim "fexp" : Float -> Float) f;
func log(f : Float) : Float = (prim "flog" : Float -> Float) f;

// Array utilities

func Array_init<T>(len : Nat, x : T) : [var T] {
  (prim "Array.init" : <T>(Nat, T) -> [var T]) <T>(len, x);
};

func Array_tabulate<T>(len : Nat, gen : Nat -> T) : [T] {
  (prim "Array.tabulate" : <T>(Nat, Nat -> T) -> [T]) <T>(len, gen);
};

func blobToArray(b : Blob) : [Nat8] = (prim "blobToArray" : (Blob) -> [Nat8]) b;
func blobToArrayMut(b : Blob) : [var Nat8] = (prim "blobToArrayMut" : (Blob) -> [var Nat8]) b;
func arrayToBlob(a : [Nat8]) : Blob = (prim "arrayToBlob" : [Nat8] -> Blob) a;
func arrayMutToBlob(a : [var Nat8]) : Blob = (prim "arrayMutToBlob" : [var Nat8] -> Blob) a;

// Error codes
type ErrorCode = {
  #system_fatal;
  #system_transient;
  #destination_invalid;
  #canister_reject;
  #canister_error;
  #future : Nat32;
  #call_error : { err_code : Nat32 };
};

// creation and inspection of abstract error
func error(message : Text) : Error {
  let e = (#canister_reject, message);
  (prim "cast" : (ErrorCode, Text) -> Error) e;
};
func errorCode(e : Error) : ErrorCode = ((prim "cast" : Error -> (ErrorCode, Text)) e).0;
func errorMessage(e : Error) : Text = ((prim "cast" : Error -> (ErrorCode, Text)) e).1;

// Time

func time() : Nat64 = (prim "time" : () -> Nat64)();

// Principal

func blobOfPrincipal(id : Principal) : Blob = (prim "blobOfPrincipal" : Principal -> Blob) id;
func principalOfBlob(act : Blob) : Principal {
  // TODO: better: check size in prim "principalOfBob" instead
  if (act.size() > 29) {
    trap("blob too long for principal");
  };
  (prim "principalOfBlob" : Blob -> Principal) act;
};

func principalOfActor(act : actor {}) : Principal = (prim "principalOfActor" : (actor {}) -> Principal) act;
func isController(p : Principal) : Bool = (prim "is_controller" : Principal -> Bool) p;
func canisterVersion() : Nat64 = (prim "canister_version" : () -> Nat64)();

// Untyped dynamic actor creation from blobs
let createActor : (wasm : Blob, argument : Blob) -> async Principal = @create_actor_helper;

func cyclesBalance() : Nat {
  (prim "cyclesBalance" : () -> Nat)();
};

func cyclesAvailable() : Nat {
  (prim "cyclesAvailable" : () -> Nat)();
};

func cyclesRefunded() : Nat {
  @refund;
};

func cyclesAccept<system>(amount : Nat) : Nat {
  (prim "cyclesAccept" : Nat -> Nat)(amount);
};

func cyclesAdd<system>(amount : Nat) : () {
  if (amount == 0) return;
  @cycles += amount;
  // trap if @cycles would exceed 2^128
  if (@cycles > 0xFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFF) {
    trap("cannot add more than 2^128 cycles");
  };
};

func cyclesBurn<system>(amount : Nat) : Nat {
  (prim "cyclesBurn" : Nat -> Nat) amount;
};

// certified data
func setCertifiedData(data : Blob) = (prim "setCertifiedData" : Blob -> ()) data;
func getCertificate() : ?Blob = (prim "getCertificate" : () -> ?Blob)();

// stable memory

/// @deprecated M0199
func stableMemorySize() : Nat64 = (prim "stableMemorySize" : () -> Nat64)();

/// @deprecated M0199
func stableMemoryGrow(pages : Nat64) : Nat64 = (prim "stableMemoryGrow" : Nat64 -> Nat64) pages;

/// @deprecated M0199
func stableMemoryLoadNat32(offset : Nat64) : Nat32 = (prim "stableMemoryLoadNat32" : Nat64 -> Nat32) offset;

/// @deprecated M0199
func stableMemoryStoreNat32(offset : Nat64, val : Nat32) : () = (prim "stableMemoryStoreNat32" : (Nat64, Nat32) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadNat8(offset : Nat64) : Nat8 = (prim "stableMemoryLoadNat8" : Nat64 -> Nat8) offset;

/// @deprecated M0199
func stableMemoryStoreNat8(offset : Nat64, val : Nat8) : () = (prim "stableMemoryStoreNat8" : (Nat64, Nat8) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadNat16(offset : Nat64) : Nat16 = (prim "stableMemoryLoadNat16" : Nat64 -> Nat16) offset;

/// @deprecated M0199
func stableMemoryStoreNat16(offset : Nat64, val : Nat16) : () = (prim "stableMemoryStoreNat16" : (Nat64, Nat16) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadNat64(offset : Nat64) : Nat64 = (prim "stableMemoryLoadNat64" : Nat64 -> Nat64) offset;

/// @deprecated M0199
func stableMemoryStoreNat64(offset : Nat64, val : Nat64) : () = (prim "stableMemoryStoreNat64" : (Nat64, Nat64) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadInt32(offset : Nat64) : Int32 = (prim "stableMemoryLoadInt32" : Nat64 -> Int32) offset;

/// @deprecated M0199
func stableMemoryStoreInt32(offset : Nat64, val : Int32) : () = (prim "stableMemoryStoreInt32" : (Nat64, Int32) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadInt8(offset : Nat64) : Int8 = (prim "stableMemoryLoadInt8" : Nat64 -> Int8) offset;

/// @deprecated M0199
func stableMemoryStoreInt8(offset : Nat64, val : Int8) : () = (prim "stableMemoryStoreInt8" : (Nat64, Int8) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadInt16(offset : Nat64) : Int16 = (prim "stableMemoryLoadInt16" : Nat64 -> Int16) offset;

/// @deprecated M0199
func stableMemoryStoreInt16(offset : Nat64, val : Int16) : () = (prim "stableMemoryStoreInt16" : (Nat64, Int16) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadInt64(offset : Nat64) : Int64 = (prim "stableMemoryLoadInt64" : Nat64 -> Int64) offset;

/// @deprecated M0199
func stableMemoryStoreInt64(offset : Nat64, val : Int64) : () = (prim "stableMemoryStoreInt64" : (Nat64, Int64) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadFloat(offset : Nat64) : Float = (prim "stableMemoryLoadFloat" : Nat64 -> Float) offset;

/// @deprecated M0199
func stableMemoryStoreFloat(offset : Nat64, val : Float) : () = (prim "stableMemoryStoreFloat" : (Nat64, Float) -> ())(offset, val);

/// @deprecated M0199
func stableMemoryLoadBlob(offset : Nat64, size : Nat) : Blob = (prim "stableMemoryLoadBlob" : (Nat64, Nat) -> Blob)(offset, size);

/// @deprecated M0199
func stableMemoryStoreBlob(offset : Nat64, val : Blob) : () = (prim "stableMemoryStoreBlob" : (Nat64, Blob) -> ())(offset, val);

// Returns a query that computes the current actor's stable variable statistics (for now, the current size, in bytes, of serialized stable variable data).
func stableVarQuery() : shared query () -> async { size : Nat64 } = (prim "stableVarQuery" : () -> (shared query () -> async { size : Nat64 }))();

// stable regions

func regionNew() : Region =
  (prim "regionNew" : () -> Region) ();

func regionId(r : Region) : Nat =
  (prim "regionId" : Region -> Nat) r;

func regionSize(r : Region) : Nat64 =
  (prim "regionSize" : Region -> Nat64) r;

func regionGrow(r : Region, pages : Nat64) : Nat64 =
  (prim "regionGrow" : (Region, Nat64) -> Nat64) (r, pages);

func regionLoadNat32(r : Region, offset : Nat64) : Nat32 =
  (prim "regionLoadNat32" : (Region, Nat64) -> Nat32) (r, offset);

func regionStoreNat32(r : Region, offset : Nat64, val : Nat32) : () =
  (prim "regionStoreNat32" : (Region, Nat64, Nat32) -> ()) (r, offset, val);

func regionLoadNat8(r : Region, offset : Nat64) : Nat8 =
  (prim "regionLoadNat8" : (Region, Nat64) -> Nat8) (r, offset);

func regionStoreNat8(r : Region, offset : Nat64, val : Nat8) : () =
  (prim "regionStoreNat8" : (Region, Nat64, Nat8) -> ()) (r, offset, val);

func regionLoadNat16(r : Region, offset : Nat64) : Nat16 =
  (prim "regionLoadNat16" : (Region, Nat64) -> Nat16) (r, offset);

func regionStoreNat16(r : Region, offset : Nat64, val : Nat16) : () =
  (prim "regionStoreNat16" : (Region, Nat64, Nat16) -> ()) (r, offset, val);

func regionLoadNat64(r : Region, offset : Nat64) : Nat64 =
  (prim "regionLoadNat64" : (Region, Nat64) -> Nat64) (r, offset);

func regionStoreNat64(r : Region, offset : Nat64, val : Nat64) : () =
  (prim "regionStoreNat64" : (Region, Nat64, Nat64) -> ()) (r, offset, val);

func regionLoadInt32(r : Region, offset : Nat64) : Int32 =
  (prim "regionLoadInt32" : (Region, Nat64) -> Int32) (r, offset);

func regionStoreInt32(r : Region, offset : Nat64, val : Int32) : () =
  (prim "regionStoreInt32" : (Region, Nat64, Int32) -> ()) (r, offset, val);

func regionLoadInt8(r : Region, offset : Nat64) : Int8 =
  (prim "regionLoadInt8" : (Region, Nat64) -> Int8) (r, offset);

func regionStoreInt8(r : Region, offset : Nat64, val : Int8) : () =
  (prim "regionStoreInt8" : (Region, Nat64, Int8) -> ()) (r, offset, val);

func regionLoadInt16(r : Region, offset : Nat64) : Int16 =
  (prim "regionLoadInt16" : (Region, Nat64) -> Int16) (r, offset);

func regionStoreInt16(r : Region, offset : Nat64, val : Int16) : () =
  (prim "regionStoreInt16" : (Region, Nat64, Int16) -> ()) (r, offset, val);

func regionLoadInt64(r : Region, offset : Nat64) : Int64 =
  (prim "regionLoadInt64" : (Region, Nat64) -> Int64) (r, offset);

func regionStoreInt64(r : Region, offset : Nat64, val : Int64) : () =
  (prim "regionStoreInt64" : (Region, Nat64, Int64) -> ()) (r, offset, val);

func regionLoadFloat(r : Region, offset : Nat64) : Float =
  (prim "regionLoadFloat" : (Region, Nat64) -> Float) (r, offset);

func regionStoreFloat(r : Region, offset : Nat64, val :  Float) : () =
  (prim "regionStoreFloat" : (Region, Nat64, Float) -> ()) (r, offset, val);

func regionLoadBlob(r : Region, offset : Nat64, size : Nat) : Blob =
  (prim "regionLoadBlob" : (Region, Nat64, Nat) -> Blob) (r, offset, size);

func regionStoreBlob(r : Region, offset : Nat64, val :  Blob) : () =
  (prim "regionStoreBlob" : (Region, Nat64, Blob) -> ()) (r, offset, val);


let call_raw = @call_raw;

func performanceCounter(counter : Nat32) : Nat64 = (prim "performanceCounter" : (Nat32) -> Nat64) counter;

// Candid configuration
func setCandidLimits<system> (
  { numerator:  Nat32;
    denominator:  Nat32;
    bias: Nat32 }
  ) {
  (prim "setCandidLimits" : (Nat32, Nat32, Nat32) -> ())
    (numerator, denominator, bias)
};

func getCandidLimits<system>() :
  { numerator:  Nat32;
    denominator:  Nat32;
    bias: Nat32 } {
  let (numerator, denominator, bias) = (prim "getCandidLimits" : () -> (Nat32, Nat32, Nat32)) ();
  { numerator;
    denominator;
    bias }
};

// predicates for motoko-san

func forall<T>(f: T -> Bool): Bool {
  (prim "forall" : <T>(T -> Bool) -> Bool) <T>(f);
};

func exists<T>(f: T -> Bool): Bool {
  (prim "exists" : <T>(T -> Bool) -> Bool) <T>(f);
};

func Ret<T>(): T {
  (prim "viperRet" : <T>() -> T) ();
};
