open Mo_types

type t = {
  typ : Type.typ;
  eff : Type.eff;
  const : bool;
  check_run : int;
  gadt_sigma : Type.typ Type.ConEnv.t option;
}

val def : t

