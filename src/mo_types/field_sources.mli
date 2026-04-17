open Source

module Srcs_tbl : Hashtbl.S with type key = region
type srcs_tbl = Region_set.t Srcs_tbl.t
type t = srcs_tbl

module Srcs_map : sig
   include module type of Region_map with type key = region

   val adjoin : Region_set.t t -> Region_set.t t -> Region_set.t t
end
type srcs_map = Region_set.t Srcs_map.t

val empty_srcs_tbl : unit -> t
val get_srcs : t -> region -> Region_set.t
val add_src : t -> region -> unit
val of_immutable_map : srcs_map -> t
val of_mutable_tbl : t -> srcs_map
