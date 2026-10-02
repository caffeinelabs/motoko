open Mo_def
open Mo_types

type uncovered = string
type unreached = Source.region

val check_pat : Syntax.pat -> Type.typ -> uncovered list * unreached list
val check_cases : Syntax.case list -> Type.typ -> uncovered list * unreached list
