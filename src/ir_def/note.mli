open Mo_types

type t = {
  typ : Type.typ;
  eff : Type.eff;
  const : bool;
  check_run : int;
}

val def : t

