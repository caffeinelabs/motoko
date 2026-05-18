open Mo_types

type t = {
  typ : Type.typ;
  eff : Type.eff;
  const : bool;
  check_run : int;
}

let def : t = {
  typ = Type.Pre;
  eff = Type.Triv;
  const = false;
  check_run = 0;
}

