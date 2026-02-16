(* A common data type for diagnostic messages *)

type severity
type error_code = string
type priority = Primary | Secondary
type span = {
  prio : priority;
  at_span : Source.region;
  label : string;
}
type message = {
  sev : severity;
  code : error_code;
  at : Source.region;
  cat : string;
  text : string;
  spans : span list;
  notes: string list;
}

type messages = message list

val info_message : Source.region -> string -> ?spans:span list -> ?notes:string list -> string -> message
val warning_message : Source.region -> error_code -> string -> ?spans:span list -> ?notes:string list -> string -> message
val error_message : Source.region -> error_code -> string -> ?spans:span list -> ?notes:string list -> string -> message

val string_of_message : message -> string
val is_treated_as_error : message -> bool
val print_messages : messages -> unit

(*
An extension of the built-in result type that also reports diagnostic messages.
Both success and failure can come with messages)
*)

type 'a result = ('a * messages, messages) Stdlib.result

val info : Source.region -> string -> string -> unit result
val warn : Source.region -> error_code -> string -> string -> unit result
val error : Source.region -> error_code -> string -> string -> 'a result

val return : 'a -> 'a result
val bind : 'a result -> ('a -> 'b result) -> 'b result
val map : ('a -> 'b) -> 'a result -> 'b result
val traverse : ('a -> 'b result) -> 'a list -> 'b list result
val traverse_ : ('a -> unit result) -> 'a list -> unit result
val fold : ('a -> 'b -> 'a result) -> 'a -> 'b list -> 'a result
val flush_messages : 'a result -> 'a option
val finally : (unit -> unit) -> 'a result -> 'a result
val run : 'a result -> 'a (* Prints messages, and exits upon failure *)

module Syntax : sig
  val (let*) : 'a result -> ('a -> 'b result) -> 'b result
end

(*
An impure, but more more convenient interface.

The 'result' type above is a monad, and would be sufficient to model, e.g., the
type checker. But since monadic style is cumbersome, the following definitions
provide an impure way of tracking messages.

The function with_message_store returns Error if its argument returns None or
the reported messages contain an error.
*)

type msg_store
val is_error_free : msg_store -> bool
val add_msg : msg_store -> message -> unit
val add_msgs : msg_store -> messages -> unit
val with_message_store : ?allow_errors:bool -> (msg_store -> 'a option) -> 'a result
