open Ir
open Source
let rec is_irrefutable p = match p.it with
  | TupP pats -> List.for_all is_irrefutable pats
  | ObjP pfs -> List.for_all (fun (pf : pat_field) -> is_irrefutable pf.it.pat) pfs
  | AltP (pat1, _) -> is_irrefutable pat1
  | AndP (pat1, pat2) -> is_irrefutable pat1 && is_irrefutable pat2
  | WildP | VarP _ -> true
  | TagP _ | LitP _ | OptP _ -> false

let is_irrefutable_nonbinding p =
  is_irrefutable p && Freevars.(M.is_empty (pat p))
