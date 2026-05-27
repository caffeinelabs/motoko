// overlap inside a switch case — `gather_pat` is not the first line of
// defence here, so this hits the M0260 check we added in `check_pat`
func f(v : Nat) : Nat =
  switch v {
    case ((x : Nat) and (x : Nat)) x;
  };
ignore f 3
