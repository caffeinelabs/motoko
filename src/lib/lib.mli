(* Things that should be in the OCaml library... *)

module Format :
sig
  (* Pretty-print to infinite line *)
  val with_str_formatter : (Format.formatter -> 'a -> unit) -> 'a -> string

  (* Display input on newline vertically indented 2 spaces *)
  val display : (Format.formatter -> 'a -> unit) -> Format.formatter -> 'a -> unit

end

module Fun :
sig
  val curry : ('a * 'b -> 'c) -> ('a -> 'b -> 'c)
  val uncurry : ('a -> 'b -> 'c) -> ('a * 'b -> 'c)

  val repeat : int -> ('a -> unit) -> 'a -> unit
end

type ('a, 'b) these =
  | This of 'a
  | That of 'b
  | Both of 'a * 'b

module List :
sig
  val equal : ('a -> 'a -> bool) -> 'a list -> 'a list -> bool
  val make : int -> 'a -> 'a list
  val table : int -> (int -> 'a) -> 'a list
  val group : ('a -> 'a -> bool) -> 'a list -> 'a list list
  val take : int -> 'a list -> 'a list (* raises Failure *)
  val drop : int -> 'a list -> 'a list (* raises Failure *)
  val replicate : 'a -> int -> 'a list
  val split_at : int -> 'a list -> ('a list * 'a list)
  val split3 : ('a * 'b * 'c) list -> ('a list * 'b list * 'c list)
  val mapi2 : (int -> 'a -> 'b -> 'c) -> 'a list -> 'b list -> 'c list

  val hd_opt : 'a list -> 'a option
  val last : 'a list -> 'a (* raises Failure *)
  val last_opt : 'a list -> 'a option
  val split_last : 'a list -> 'a list * 'a (* raises Failure *)

  val index_of : 'a -> 'a list -> int option
  val index_where : ('a -> bool) -> 'a list -> int option

  val compare : ('a -> 'a -> int) -> 'a list -> 'a list -> int
  val is_ordered : ('a -> 'a -> int) -> 'a list -> bool
  val is_strictly_ordered : ('a -> 'a -> int) -> 'a list -> bool
  val is_prefix : ('a -> 'a -> bool) -> 'a list -> 'a list -> bool

  val iter_pairs : ('a -> 'a -> unit) -> 'a list -> unit

  val safe_map : ('a -> 'b) -> 'a list -> 'b list

  (** Assumes the two lists have been sorted with respect to the comparison function *)
  val align : ('a -> 'b -> int) -> 'a list -> 'b list -> ('a, 'b) these Seq.t
end

module List32 :
sig
  val make : int32 -> 'a -> 'a list
  val length : 'a list -> int32
  val nth : 'a list -> int32 -> 'a (* raises Failure *)
  val take : int32 -> 'a list -> 'a list (* raises Failure *)
  val drop : int32 -> 'a list -> 'a list (* raises Failure *)
end

module Array :
sig
  val compare : ('a -> 'a -> int) -> 'a array -> 'a array -> int
  val for_all2 : ('a -> 'b -> bool) -> 'a array -> 'b array -> bool (* raises Failure *)
end

module Array32 :
sig
  val make : int32 -> 'a -> 'a array
  val length : 'a array -> int32
  val get : 'a array -> int32 -> 'a
  val set : 'a array -> int32 -> 'a -> unit
  val blit : 'a array -> int32 -> 'a array -> int32 -> int32 -> unit
end

module Bigarray :
sig
  open Bigarray

  module Array1_64 :
  sig
    val create : ('a, 'b) kind -> 'c layout -> int64 -> ('a, 'b, 'c) Array1.t
    val dim : ('a, 'b, 'c) Array1.t -> int64
    val get : ('a, 'b, 'c) Array1.t -> int64 -> 'a
    val set : ('a, 'b, 'c) Array1.t -> int64 -> 'a -> unit
    val sub : ('a, 'b, 'c) Array1.t -> int64 -> int64 -> ('a, 'b, 'c) Array1.t
  end
end

module Option :
sig
  module Syntax :
  sig
    val (let*) : 'a option -> ('a -> 'b option) -> 'b option
    val (let+) : 'a option -> ('a -> 'b) -> 'b option
    val (and+) : 'a option -> 'b option -> ('a * 'b) option
  end
  val get : 'a option -> 'a -> 'a
  val exists : ('a -> bool) -> 'a option -> bool
  val map2 : ('a -> 'b -> 'c) -> 'a option -> 'b option -> 'c option
end

module Promise :
sig
  type 'a t
  exception Promise
  val make : unit -> 'a t
  val make_fulfilled : 'a -> 'a t
  val fulfill : 'a t -> 'a -> unit
  val is_fulfilled : 'a t -> bool
  val value : 'a t -> 'a
  val value_opt : 'a t -> 'a option
  val lazy_value : 'a t -> (unit -> 'a) -> 'a
end

module AllocOnUse :
sig
  type ('a, 'b) t
  val make : (unit -> ('a * ('b -> unit))) -> ('a, 'b) t
  val use : ('a, 'b) t -> 'a
  val def : ('a, 'b) t -> ('b Lazy.t) -> unit
end

module Int :
sig
  val log2 : int -> int
  val is_power_of_two : int -> bool
end

module Uint32 :
sig
  type t
  val to_string : t -> string
  val of_string : string -> t
  val of_string_opt : string -> t option
  val of_int : int -> t
  val to_int : t -> int
  val of_int32 : int32 -> t
  val to_int32 : t -> int32
  val add : t -> t -> t
  val sub : t -> t -> t
  val mul : t -> t -> t
  val succ : t -> t
  val zero : t
  val one : t
  val compare : t -> t -> int
  val logand : t -> t -> t
  val logor : t -> t -> t
  val shift_right_logical : t -> int -> t
end

module String :
sig
  val implode : char list -> string
  val explode : string -> char list
  val split : string -> char -> string list
  val breakup : string -> int -> string list
  val find_from_opt : (char -> bool) -> string -> int -> int option
  val chop_prefix : string -> string -> string option
  val chop_suffix : string -> string -> string option
  val lightweight_escaped : string -> string
  val levenshtein_distance : string -> string -> int
end

module CRC :
sig
  val crc8 : string -> int
  val crc32 : string -> int32
end

module Hex :
sig
  val hexdigit : char -> int
  val bytes_of_hex : string -> string
  val int_of_hex_byte : string -> int

  val hex_of_byte  : int -> string
  val hex_of_char  : char -> string
  val hex_of_bytes : string -> string
end

module Base32 :
sig
  val decode : string -> (string, string) result
  val encode : string -> string
end

module Utf8 :
sig
  type t = int list
  exception Utf8
  val is_valid : string -> bool
  val decode : string -> t (* raises Utf8 *)
  val encode : t -> string (* raises Utf8 *)
end

module FilePath :
sig
  (**
   * Normalises a file path
   *)
  val normalise : string -> string

  (**
   * Makes one path relative to another path.
   *
   * Examples:
   *
   * relative_to "/home/foo" "/home/foo/project" = Some "project"
   * relative_to "/home/foo" "/home/foo/project/lib" = Some "project/lib"
   * relative_to "/home/foo" "/home/bar/project" = None
   * relative_to "foo/bar" "foo/bar/project" = Some "project"
   *)
  val relative_to : string -> string -> string option

  val make_absolute : string -> string -> string

  (**
   * Checks whether one path is nested below another.
   * Must only be called on two absolute paths!
   *
   * Examples:
   *
   * is_subpath "/home" "/home/path" = true
   * is_subpath "/home" "/path" = false
   * is_subpath "/home/path" "/home" = false
   * is_subpath "/home" "/homepath" = false
   *)
  val is_subpath : string -> string -> bool


  (**
   * Opens a file, and if successful checks whether there were any mismatches
   * in filename case (in case-insensitive file systems), returning a warning.
   *
   * Examples:
   *
   * Asked for "Array.mo", opened "array.mo" reports
   * "warning, file Array.mo has been located with a name of different case"
   *)
  val open_in : string -> (in_channel * string list)


  (**
   * Reads all bytes from the indicated file
   *)
  val contents : string -> string
end
