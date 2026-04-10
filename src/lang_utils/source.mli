type pos = Wasm.Source.pos = { file : string; line : int; column : int }
type region = Wasm.Source.region = { left : pos; right : pos }
type ('a, 'b) annotated_phrase = {at : region; it : 'a; mutable note: 'b}
type 'a phrase = ('a, unit) annotated_phrase

module Pos_ord : Set.OrderedType with type t = pos
module Region_ord : Set.OrderedType with type t = region

module Region_set : Set.S with type elt = region
module Region_map : Map.S with type key = region

val no_pos : pos
val no_region : region

val is_no_pos : pos -> bool
val is_no_region : region -> bool

val string_of_pos : pos -> string
val string_of_region : region -> string

val span : region -> region -> region
val between : region -> region -> region

val annotate : 'b -> 'a -> region -> ('a, 'b) annotated_phrase
val (@@) : 'a -> region -> 'a phrase

val read_region_with_markers : region -> string option
val read_region : region -> string option

exception ParseError of region * string
