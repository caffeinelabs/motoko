//MOC-ENV MOC_UNLOCK_PRIM=yesplease
import Prim "mo:⛔";

// `Prim.ErrorCode` must equal the prelude's `@ErrorCode`: each flows into the other
func _check(e : Prim.ErrorCode, f : @ErrorCode) : (e : Prim.ErrorCode, f : @ErrorCode) = (f, e)
