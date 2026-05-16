open Mo_types

(* M11a: GADT existential refinement σ may be attached to an IR
   exp's note. Replaces the region-keyed side-table for
   construction-side σ (CallPrim / TagPrim / TupPrim / ObjE /
   ObjBlockE). σ at None means "no refinement"; reading falls
   back to the side-table during migration. *)
type t = {
  typ : Type.typ;
  eff : Type.eff;
  const : bool;
  check_run : int;
  gadt_sigma : Type.typ Type.ConEnv.t option;
}

let def : t = {
  typ = Type.Pre;
  eff = Type.Triv;
  const = false;
  check_run = 0;
  gadt_sigma = None;
}

