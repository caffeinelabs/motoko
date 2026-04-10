(* Used by the language server to help track possible sources of an object
   field. As objects in Motoko are structural rather than nominal and may be
   subject to subtyping, this helps to overapproximate these sources.

   Note that the functionality of this module is enabled only when
   [!Mo_config.Flags.typechecker_combine_srcs = true], otherwise the functions
   will do nothing or return empty data structures. *)

open Source

module Srcs_tbl = Hashtbl.Make (struct
  type t = region

  let equal l r = l = r

  let hash s =
    let combine_int h x = (h * 65521) lxor x in
    let hash_pos {line; column; file} =
      combine_int line (combine_int column (Int32.to_int (Hash.hash file)))
    in
    combine_int (hash_pos s.left) (hash_pos s.right)
end)

type srcs_tbl = Region_set.t Srcs_tbl.t
type t = srcs_tbl

module Srcs_map = struct
  include Region_map

  let adjoin = union (fun _ rs1 rs2 -> Some (Region_set.union rs1 rs2))
end

type srcs_map = Region_set.t Srcs_map.t

let empty_srcs_tbl () =
  let initial_size =
    if !Mo_config.Flags.typechecker_combine_srcs then 1023 else 0
  in
  Srcs_tbl.create initial_size

let get_srcs srcs_tbl r =
  if !Mo_config.Flags.typechecker_combine_srcs then
    Option.value
      ~default:(Region_set.singleton r)
      (Srcs_tbl.find_opt srcs_tbl r)
  else
    Region_set.empty

let add_src srcs_tbl region =
  if !Mo_config.Flags.typechecker_combine_srcs then
    let srcs =
      match Srcs_tbl.find_opt srcs_tbl region with
      | None -> Region_set.singleton region
      | Some srcs -> Region_set.add region srcs
    in
    Srcs_tbl.replace srcs_tbl region srcs

let of_immutable_map srcs_map =
  srcs_map
  |> Region_map.to_seq
  |> Srcs_tbl.of_seq

let of_mutable_tbl srcs_tbl =
  srcs_tbl
  |> Srcs_tbl.to_seq
  |> Region_map.of_seq
