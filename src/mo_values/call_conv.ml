open Mo_types
open Type

(*
Functions of different arities (even if the look like they have the same type)
are not compatible, but the interpreters just pass tuples to them. In order to
still catch mismatching arities in the interpreter, we tag function values
with their “calling convention”, and check them in calls.
*)

type call_conv = {
  sort: func_sort;
  control : control;
  n_args : int;
  n_res : int;
}
type t = call_conv

let local_cc n m = { sort = Local; control = Returns; n_args = n; n_res = m}
let message_cc s n = { sort = Shared s; control = Returns; n_args = n; n_res = 0}
let async_cc s n m = { sort = Shared s; control = Promises; n_args = n; n_res = m}
let replies_cc s n m = { sort = Shared s; control = Replies; n_args = n; n_res = m}

let call_conv_of_typ typ =
  match promote typ with
  | Func (sort, control, tbds, dom, res) ->
    { sort; control; n_args = List.length dom; n_res = List.length res }
  | Non ->
    { sort = Local; control = Returns; n_args = 1; n_res = 1 }
  | _ -> raise (Invalid_argument ("call_conv_of_typ " ^ string_of_typ typ))

let string_of_call_conv {sort;control;n_args;n_res} =
  Printf.sprintf "(%s%i %s %i)"
    (string_of_func_sort sort)
    n_args
    (match control with Returns -> "->" | Promises -> "@>" | Replies -> "#>"
     | Intrinsic id -> "intrinsic " ^ id)
    n_res

