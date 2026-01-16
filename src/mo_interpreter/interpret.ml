open Mo_def
open Mo_values
open Mo_types
open Mo_config

open Syntax
open Source

module V = Value
module T = Type
module CC = Call_conv

(* Context *)

type val_env = V.def V.Env.t
type lib_env = V.value V.Env.t
type lab_env = V.value V.cont V.Env.t
type ret_env = V.value V.cont option
type throw_env = V.value V.cont option
type actor_env = V.value V.Env.t ref (* indexed by actor ids *)

(* The actor heap.
    NB: A cut-down ManagementCanister with id "" is added later, to enjoy access to logging facilities.
*)
let state = ref V.Env.empty

type flags =
  { trace : bool;
    print_depth : int
  }

type scope =
  { val_env: V.def V.Env.t;
    lib_env: V.value V.Env.t;
  }

type env =
  { flags : flags;
    vals : val_env;
    labs : lab_env;
    libs : lib_env;
    rets : ret_env;
    throws : throw_env;
    self : V.actor_id;
    actor_env : actor_env;
  }

let adjoin_scope scope1 scope2 =
  { val_env = V.Env.adjoin scope1.val_env scope2.val_env;
    lib_env = V.Env.adjoin scope1.lib_env scope2.lib_env;
  }

let adjoin_vals env ve = { env with vals = V.Env.adjoin env.vals ve }

let empty_scope = { val_env = V.Env.empty; lib_env = V.Env.empty }

let lib_scope f v scope : scope =
  { scope with lib_env = V.Env.add f v scope.lib_env }

let env_of_scope flags ae scope =
  { flags;
    vals = scope.val_env;
    libs = scope.lib_env;
    labs = V.Env.empty;
    rets = None;
    throws = None;
    self = V.top_id;
    actor_env = ae;
  }

let context env = V.Blob env.self

(* Error handling *)

exception Trap of Source.region * string
exception Cancel of string

let trap at fmt = Printf.ksprintf (fun s -> raise (Trap (at, s))) fmt

let find id env =
  try V.Env.find id env
  with Not_found ->
    let dom = V.Env.keys env in
    trap no_region "unbound identifier %s in domain %s" id (String.concat " " dom)

let lookup_actor env at aid id =
  match V.Env.find_opt aid !(env.actor_env) with
  | None -> trap at "Unknown actor \"%s\"" aid
  | Some actor_value ->
     let fs = V.as_obj actor_value in
     match V.Env.find_opt id fs with
     | None -> trap at "Actor \"%s\" has no method \"%s\"" aid id
     | Some field_value -> field_value

(* Tracing *)

let trace_depth = ref 0

let trace fmt =
  Printf.ksprintf (fun s ->
    Printf.printf "%s%s\n%!" (String.make (2 * !trace_depth) ' ') s
  ) fmt

let string_of_val env = V.string_of_val env.flags.print_depth T.Non
let string_of_def flags = V.string_of_def flags.print_depth T.Non
let string_of_arg env = function
  | V.Tup _ as v -> string_of_val env v
  | v -> "(" ^ string_of_val env v ^ ")"


(* Debugging aids *)

let last_env = ref (env_of_scope {trace = false; print_depth = 2} state empty_scope)
let last_region = ref Source.no_region

let print_exn flags exn =
  let trace = Printexc.get_backtrace () in
  Printf.printf "%!";
  let at = Source.string_of_region !last_region in
  Printf.eprintf "%s: internal error, %s\n" at (Printexc.to_string exn);
  Printf.eprintf "\nLast environment:\n";
  Value.Env.iter (fun x d -> Printf.eprintf "%s = %s\n" x (string_of_def flags d))
    !last_env.vals;
  Printf.eprintf "\n";
  Printf.eprintf "%s" trace;
  Printf.eprintf "%!"

(* Scheduling *)

let step_total = ref 0
let step_limit = ref 0

module Scheduler =
struct
  let q : (unit -> unit) Queue.t = Queue.create ()

  let queue work = Queue.add work q
  let yield () =
    trace_depth := 0;
    try Queue.take q () with Trap (at, msg) ->
      Printf.eprintf "%s: execution error, %s\n" (Source.string_of_region at) msg
  let rec run () =
    if not (Queue.is_empty q) then (yield (); run ())

  let tmp : (unit -> unit) Queue.t = Queue.create ()
  let bounce work =
    (* add work to *front* of queue *)
    Queue.transfer q tmp;
    Queue.add work q;
    Queue.transfer tmp q

  let interval = 128
  let count = ref interval
  let trampoline3 f x y z =
    if !Flags.ocaml_js then begin
        step_total := (!step_total) + 1;
        if !step_total = !step_limit then raise (Cancel "interpreter reached step limit");
        if !count <= 0 then begin
            count := interval;
            bounce (fun () -> f x y z);
          end
        else begin
            count := (!count) - 1;
            f x y z
          end
      end
    else f x y z
end


(* Async auxiliary functions *)

let make_async () : V.async =
  {V.result = Lib.Promise.make (); waiters = []}

let get_async async (k : V.value V.cont) (r : V.value V.cont) =
  match Lib.Promise.value_opt async.V.result with
  | Some (V.Ok v) -> k v
  | Some (V.Error v) -> r v
  | None -> async.V.waiters <- (k,r)::async.V.waiters

let set_async async v =
  List.iter (fun (k,_) -> Scheduler.queue (fun () -> k v)) async.V.waiters;
  Lib.Promise.fulfill async.V.result (V.Ok v);
  async.V.waiters <- []

let reject_async async v =
  List.iter (fun (_,k) -> Scheduler.queue (fun () -> k v)) async.V.waiters;
  Lib.Promise.fulfill async.V.result (V.Error v);
  async.V.waiters <- []

let reply async v =
  Scheduler.queue (fun () -> set_async async v)

let reject async v =
  match v with
  | V.Tup [ _code; message ] ->
    (* mask the error code before rejecting *)
    Scheduler.queue
      (fun () -> reject_async async (V.Tup [V.Variant("canister_reject", V.unit); message]))
  | _ -> assert false

let async env at (f: (V.value V.cont) -> (V.value V.cont) -> unit) (k : V.value V.cont) =
    let async = make_async () in
    let k' = reply async in
    let r  = reject async in
    if env.flags.trace then trace "-> async %s" (string_of_region at);
    Scheduler.queue (fun () ->
      if env.flags.trace then trace "<- async %s" (string_of_region at);
      incr trace_depth;
      f (fun v ->
        if env.flags.trace then trace "<= %s" (string_of_val env v);
        decr trace_depth;
        k' v)
        r
    );
    k (V.Async async)

let await env at short async k =
  let adorn, schedule = if short then "?", (|>) () else "", Scheduler.queue in
  if env.flags.trace then trace "=> await%s %s" adorn (string_of_region at);
  decr trace_depth;
  get_async async (fun v ->
      schedule (fun () ->
        if env.flags.trace then
          trace "<- await%s %s%s" adorn (string_of_region at) (string_of_arg env v);
        incr trace_depth;
        k v
      )
    )
    (let r = Option.get env.throws in
     fun v ->
       schedule (fun () ->
         if env.flags.trace then
           trace "<- await%s %s threw %s" adorn (string_of_region at) (string_of_arg env v);
         incr trace_depth;
         r v))

let make_unit_message env id v =
  let open CC in
  let call_conv, f = V.as_func v in
  match call_conv with
  | {sort = T.Shared s; n_res = 0; _} ->
    Value.message_func s call_conv.n_args f
  | _ -> (* assert false *)
    failwith ("unexpected call_conv " ^ (string_of_call_conv call_conv))

let make_async_message env id v =
  let open CC in
  let call_conv, f = V.as_func v in
  match call_conv with
  | {sort = T.Shared s; control = T.Promises; _} ->
    Value.async_func s call_conv.n_args call_conv.n_res f
  | _ -> (* assert false *)
    failwith ("unexpected call_conv " ^ (string_of_call_conv call_conv))


let make_message env name t v : V.value =
  match t with
  | T.Func (_, T.Returns, _, _, _) -> make_unit_message env name v
  | T.Func (_, T.Promises, _, _, _) -> make_async_message env name v
  | _ -> (* assert false *)
    failwith (Printf.sprintf "actorfield: %s %s" name (T.string_of_typ t))


(* Literals *)

let interpret_lit env lit : V.value =
  match !lit with
  | NullLit -> V.Null
  | BoolLit b -> V.Bool b
  | NatLit n -> V.Int n
  | Nat8Lit n -> V.Nat8 n
  | Nat16Lit n -> V.Nat16 n
  | Nat32Lit n -> V.Nat32 n
  | Nat64Lit n -> V.Nat64 n
  | IntLit i -> V.Int i
  | Int8Lit i -> V.Int8 i
  | Int16Lit i -> V.Int16 i
  | Int32Lit i -> V.Int32 i
  | Int64Lit i -> V.Int64 i
  | FloatLit f -> V.Float f
  | CharLit c -> V.Char c
  | TextLit s -> V.Text s
  | BlobLit b -> V.Blob b
  | PreLit _ -> assert false


(* Overloaded dot implementations *)

let array_get a at =
  V.local_func 1 1 (fun c v k ->
    let n = V.as_int v in
    if Numerics.Nat.lt n (Numerics.Nat.of_int (Array.length a))
    then k (a.(Numerics.Nat.to_int n))
    else trap at "array index out of bounds"
  )

let array_put a at =
  V.local_func 2 0 (fun c v k ->
    let v1, v2 = V.as_pair v in
    let n = V.as_int v1 in
    if Numerics.Nat.lt n (Numerics.Nat.of_int (Array.length a))
    then k (a.(Numerics.Nat.to_int n) <- v2; V.Tup [])
    else trap at "array index out of bounds"
  )

let array_size a at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    k (V.Int (Numerics.Nat.of_int (Array.length a)))
  )

let array_keys a at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    let i = ref 0 in
    let next =
      V.local_func 0 1 (fun c v k' ->
        if !i = Array.length a
        then k' V.Null
        else let v = V.Opt (V.Int (Numerics.Nat.of_int !i)) in incr i; k' v
      )
    in k (V.Obj (V.Env.singleton "next" next))
  )

let array_vals a at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    let i = ref 0 in
    let next =
      V.local_func 0 1 (fun c v k' ->
        if !i = Array.length a
        then k' V.Null
        else
          let wi = match a.(!i) with
            | V.Mut r -> !r
            | w -> w in
          let v = V.Opt wi in incr i; k' v
      )
    in k (V.Obj (V.Env.singleton "next" next))
  )

let blob_vals t at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    let i = ref 0 in
    let next =
      V.local_func 0 1 (fun c v k' ->
        if !i = String.length t
        then k' V.Null
        else let v = V.Opt V.(Nat8 (Numerics.Nat8.of_int (Char.code (String.get t !i)))) in incr i; k' v
      )
    in k (V.Obj (V.Env.singleton "next" next))
  )

let blob_get t at =
  V.local_func 1 1 (fun c v k ->
    let n = V.as_int v in
    if Numerics.Nat.lt n (Numerics.Nat.of_int (String.length t))
    then k V.(Nat8 (Numerics.Nat8.of_int (Char.code (String.get t (Numerics.Nat.to_int n)))))
    else trap at "blob index out of bounds"
  )

let blob_size t at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    k (V.Int (Numerics.Nat.of_int (String.length t)))
  )

let blob_keys t at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    let i = ref 0 in
    let next =
      V.local_func 0 1 (fun c v k' ->
        if !i = String.length t
        then k' V.Null
        else let v = V.Opt (V.Int (Numerics.Nat.of_int !i)) in incr i; k' v
      )
    in k (V.Obj (V.Env.singleton "next" next))
  )

let text_chars t at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    let i = ref 0 in
    let s = Lib.Utf8.decode t in
    let next =
      V.local_func 0 1 (fun c v k' ->
        if !i = List.length s
        then k' V.Null
        else let v = V.Opt (V.Char (List.nth s !i)) in incr i; k' v
      )
    in k (V.Obj (V.Env.singleton "next" next))
  )

let text_len t at =
  V.local_func 0 1 (fun c v k ->
    V.as_unit v;
    k (V.Int (Numerics.Nat.of_int (List.length (Lib.Utf8.decode t))))
  )

(* Expressions *)

let check_call_conv exp call_conv =
  let open CC in
  let exp_call_conv = call_conv_of_typ exp.note.note_typ in
  if not (exp_call_conv = call_conv) then
    failwith (Printf.sprintf
      "call_conv mismatch: function %s of type %s expecting %s, found %s"
      (Wasm.Sexpr.to_string 80 (Arrange.exp exp))
      (T.string_of_typ exp.note.note_typ)
      (string_of_call_conv exp_call_conv)
      (string_of_call_conv call_conv)
    )

let check_call_conv_arg env exp v call_conv =
  let open CC in
  if call_conv.n_args <> 1 then
  let es = try V.as_tup v with Invalid_argument _ ->
    failwith (Printf.sprintf
      "call %s: calling convention %s cannot handle non-tuple value %s"
      (Wasm.Sexpr.to_string 80 (Arrange.exp exp))
      (string_of_call_conv call_conv)
      (string_of_val env v)
    )
  in
  if List.length es <> call_conv.n_args then
    failwith (Printf.sprintf
      "call %s: calling convention %s got tuple of wrong length %s"
      (Wasm.Sexpr.to_string 80 (Arrange.exp exp))
      (string_of_call_conv call_conv)
      (string_of_val env v)
    )

let rec interpret_exp env exp (k : V.value V.cont) =
  Scheduler.trampoline3
    interpret_exp_mut env exp (function V.Mut r -> k !r | v -> k v)

and interpret_exp_mut env exp (k : V.value V.cont) =
  last_region := exp.at;
  last_env := env;
  Profiler.bump_region exp.at ;
  match exp.it with
  | HoleE (_, e) -> interpret_exp_mut env (!e) k
  | PrimE s ->
    k (V.Func (CC.call_conv_of_typ exp.note.note_typ,
       Prim.prim { Prim.trap = trap exp.at "%s" } s
    ))
  | VarE id ->
    (match id.note with
    | (_, None) ->
      begin match Lib.Promise.value_opt (find id.it env.vals) with
      | Some v -> k v
      | None -> trap exp.at "accessing identifier before its definition"
      end
    | (_, Some exp) ->
    interpret_exp_mut env exp k)
  | ImportE (f, ri) ->
    (match !ri with
    | Unresolved -> assert false
    | LibPath {path; _} ->
      k (find path env.libs)
    | ImportedValuePath path ->
      if !Mo_config.Flags.blob_import_placeholders then
        trap exp.at "blob import placeholder"
      else begin
        let contents = Lib.FilePath.contents path in
        assert T.(exp.note.note_typ = Prim Blob);
        k (V.Blob contents)
      end
    | IDLPath _ -> trap exp.at "actor import"
    | PrimPath -> k (find "@prim" env.libs)
    )
  | ImplicitLibE lib ->
    k (find lib env.libs)
  | LitE lit ->
    k (interpret_lit env lit)
  | ActorUrlE url ->
    interpret_exp env url (fun v1 ->
      let url_text = V.as_text v1 in
      match Ic.Url.decode_principal url_text with
      (* create placeholder functions (see #3683) *)
      | Ok bytes ->
        if String.length bytes > 29 then
          trap exp.at "blob too long for actor principal"
        else
          k (V.Blob bytes)
      | Error e -> trap exp.at "could not parse %S as an actor reference: %s"  (V.as_text v1) e
    )
  | UnE (ot, op, exp1) ->
    interpret_exp env exp1
      (fun v1 ->
        k (try Operator.unop op !ot v1 with Invalid_argument s -> trap exp.at "%s" s))
  | BinE (ot, exp1, op, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      interpret_exp env exp2 (fun v2 ->
        k (try Operator.binop op !ot v1 v2 with _ ->
          trap exp.at "arithmetic overflow")
      )
    )
  | ToCandidE _ -> invalid_arg "to do: ToCandidE"
  | FromCandidE _ -> invalid_arg "to do: FromCandidE"
  | ShowE (ot, exp1) ->
    interpret_exp env exp1 (fun v ->
      if Show.can_show !ot
      then k (Value.Text (Show.show_val !ot v))
      else raise (Invalid_argument "debug_show"))
  | RelE (ot, exp1, op, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      interpret_exp env exp2 (fun v2 ->
        k (Operator.relop op !ot v1 v2)
      )
    )
  | TupE exps ->
    interpret_exps env exps [] (fun vs -> k (V.Tup vs))
  | OptE exp1 ->
    interpret_exp env exp1 (fun v1 -> k (V.Opt v1))
  | DoOptE exp1 ->
    let env' = { env with labs = V.Env.add "!" k env.labs } in
    interpret_exp env' exp1 (fun v1 -> k (V.Opt v1))
  | BangE exp1 ->
    interpret_exp env exp1 (fun v1 ->
      match v1 with
      | V.Opt v2 -> k v2
      | V.Null -> find "!" env.labs v1
      | _ -> assert false)
  | ProjE (exp1, n) ->
    interpret_exp env exp1 (fun v1 -> k (List.nth (V.as_tup v1) n))
  | ObjBlockE (_exp_opt, obj_sort, (self_id_opt, _), dec_fields) ->
    (* NB: we ignore the migration expression _exp_opt *)
    interpret_obj env obj_sort.it self_id_opt dec_fields k
  | ObjE (exp_bases, exp_fields) ->
    let fields fld_env = interpret_exp_fields env exp_fields fld_env (fun env -> k (V.Obj env)) in
    let open V.Env in
    let merges =
      List.fold_left
        (merge (fun _ l r -> match l, r with | l, None -> l | None, r -> r | _ -> assert false))
        empty in
    (* remove dynamic fields not present in the type as well as overwritten fields *)
    let labs = List.map (fun (f : Syntax.exp_field) -> f.it.id.it) exp_fields in
    let tys = List.(map (fun b ->
                         T.as_obj b.note.note_typ |>
                         snd |>
                         filter (fun f -> not (mem f.T.lab labs)))) exp_bases in
    let strip vs =
      let known fs k _ = List.exists (fun { T.lab; _ } -> k = lab) fs in
      List.map2 (fun fs v -> filter (known fs) (V.as_obj v)) tys vs in
    interpret_exps env exp_bases [] (fun objs -> fields (merges (strip objs)))
  | TagE (i, exp1) ->
    interpret_exp env exp1 (fun v1 -> k (V.Variant (i.it, v1)))
  | DotE (exp1, id, _) when T.(sub exp1.note.note_typ (Obj (Actor, [], []))) ->
    interpret_exp env exp1 (fun v1 -> k V.(Tup [v1; Text id.it]))
  | DotE (exp1, id, _) ->
    interpret_exp env exp1 (fun v1 ->
      match v1 with
      | V.Obj fs ->
        k (find id.it fs)
      | V.Array vs ->
        let f = match id.it with
          | "size" -> array_size
          | "get" -> array_get
          | "put" -> array_put
          | "keys" -> array_keys
          | "vals" | "values" -> array_vals
          | s -> assert false
        in k (f vs exp.at)
      | V.Text s ->
        let f = match id.it with
          | "size" -> text_len
          | "chars" -> text_chars
          | s -> assert false
        in k (f s exp.at)
      | V.Blob b when T.sub exp1.note.note_typ (T.blob)->
        let f = match id.it with
          | "size" -> blob_size
          | "keys" -> blob_keys
          | "get" -> blob_get
          | "vals" | "values" -> blob_vals
          | s -> assert false
        in k (f b exp.at)
      | _ -> assert false
    )
  | AssignE (exp1, exp2) ->
    interpret_exp_mut env exp1 (fun v1 ->
      interpret_exp env exp2 (fun v2 ->
        V.as_mut v1 := v2; k V.unit
      )
    )
  | ArrayE (mut, exps) ->
    interpret_exps env exps [] (fun vs ->
      let vs' =
        match mut.it with
        | Var -> List.map (fun v -> V.Mut (ref v)) vs
        | Const -> vs
      in k (V.Array (Array.of_list vs'))
    )
  | IdxE (exp1, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      interpret_exp env exp2 (fun v2 ->
        k V.(let i = Numerics.Int.to_int (as_int v2) in
             match v1 with
             | Blob s ->
               Nat8 (s.[i] |> Char.code |> Numerics.Nat8.of_int)
             | _ ->
               try (as_array v1).(i)
               with Invalid_argument s -> trap exp.at "%s" s)
      )
    )
  | FuncE (name, shared_pat, _typbinds, pat, _typ, _sugar, exp2) ->
    let f = interpret_func env name shared_pat pat (fun env' -> interpret_exp env' exp2) in
    let v = V.Func (CC.call_conv_of_typ exp.note.note_typ, f) in
    let v' =
      match shared_pat.it with
      | T.Shared _ -> make_message env name exp.note.note_typ v
      | T.Local -> v
    in k v'
  | CallE (par, exp1, typs, (_, exp2)) ->
    let exp2 = !exp2 in
    interpret_par env par
      (fun v ->
        ignore (V.as_obj v);
        let exp1, exp2 = match exp1.it with
          (* Contextual dot call *)
          | DotE (exp1, _, n) when Option.is_some !n ->
             let func_exp = Option.get !n in
             let args = contextual_dot_args exp1 exp2 func_exp in
             func_exp, args
          | _ -> exp1, exp2 in
        interpret_exp env exp1 (fun v1 ->
         let v1 = begin match v1 with
                  | V.(Tup [Blob aid; Text id]) -> lookup_actor env exp1.at aid id
                  | _ -> v1
                  end in
         interpret_exp env exp2 (fun v2 ->
             let call_conv, f = V.as_func v1 in
             check_call_conv exp1 call_conv;
             check_call_conv_arg env exp v2 call_conv;
             last_region := exp.at; (* in case the following throws *)
             let c = context env in
             f c v2 k)))
  | BlockE decs ->
    let k' =
      if T.is_unit exp.note.note_typ (* TODO: peeking at types violates erasure semantics, revisit! *)
      then (fun _v -> k V.unit)
      else k
    in
    interpret_block env decs None k'
  | NotE exp1 ->
    interpret_exp env exp1 (fun v1 -> k (V.Bool (not (V.as_bool v1))))
  | AndE (exp1, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      if V.as_bool v1
      then interpret_exp env exp2 k
      else k v1
    )
  | OrE (exp1, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      if V.as_bool v1
      then k v1
      else interpret_exp env exp2 k
    )
  | IfE (exp1, exp2, exp3) ->
    interpret_exp env exp1 (fun v1 ->
      if V.as_bool v1
      then interpret_exp env exp2 k
      else interpret_exp env exp3 k
    )
  | SwitchE (exp1, cases) ->
    interpret_exp env exp1 (fun v1 ->
      interpret_cases env cases exp.at v1 k
      )
  | TryE (exp1, cases, exp2_opt) ->
    let k, env = match exp2_opt with
      | None -> k, env
      | Some exp2 ->
        let pre k v = interpret_exp env exp2 (fun v2 -> V.as_unit v2; k v) in
        pre k,
        { env with rets = Option.map pre env.rets
                 ; labs = V.Env.map pre env.labs
                 ; throws = Option.map pre env.throws } in
    let k' v1 = interpret_catches env cases exp.at v1 k in
    interpret_exp { env with throws = Some k' } exp1 k
  | WhileE (exp1, exp2) ->
    let k_continue = fun v -> V.as_unit v; interpret_exp env exp k in
    interpret_exp env exp1 (fun v1 ->
      if V.as_bool v1
      then interpret_exp env exp2 k_continue
      else k V.unit
    )
  | LoopE (exp1, None) ->
    interpret_exp env exp1 (fun v -> V.as_unit v; interpret_exp env exp k)
  | LoopE (exp1, Some exp2) ->
    interpret_exp env exp1 (fun v1 ->
      V.as_unit v1;
      interpret_exp env exp2 (fun v2 ->
        if V.as_bool v2
        then interpret_exp env exp k
        else k V.unit
      )
    )
  | ForE (pat, exp1, exp2) ->
    interpret_exp env exp1 (fun v1 ->
      let fs = V.as_obj v1 in
      let _, next = V.as_func (find "next" fs) in
      let rec k_continue = fun v ->
        V.as_unit v;
        next (context env) V.unit (fun v' ->
          match v' with
          | V.Opt v1 ->
            (match match_pat pat v1 with
            | None ->
              trap pat.at "value %s does not match pattern" (string_of_val env v')
            | Some ve ->
              interpret_exp (adjoin_vals env ve) exp2 k_continue
            )
          | V.Null -> k V.unit
          | _ -> assert false
        )
      in k_continue V.unit
    )
  | LabelE (id, _typ, exp1) ->
    let env' = {env with labs = V.Env.add id.it k env.labs} in
    Profiler.bump_label id.at id.it ;
    interpret_exp env' exp1 k
  | BreakE (id, exp1) ->
    interpret_exp env exp1 (find id.it env.labs)
  | DebugE exp1 ->
    if !Mo_config.Flags.release_mode then k V.unit else interpret_exp env exp1 k
  | RetE exp1 ->
    interpret_exp env exp1 (Option.get env.rets)
  | ThrowE exp1 ->
    interpret_exp env exp1 (Option.get env.throws)
  | AsyncE (par, T.Fut, _, exp1) ->
    interpret_par env par
      (fun v ->
        ignore (V.as_obj v);
        async env
          exp.at
          (fun k' r ->
            let env' = { env with labs = V.Env.empty; rets = Some k'; throws = Some r }
            in interpret_exp env' exp1 k')
          k)
  | AsyncE (par, T.Cmp, _, exp1) ->
    assert (par = None);
    k (V.Comp (fun k' r ->
      let env' = {env with labs = V.Env.empty; rets = Some k'; throws = Some r}
      in interpret_exp env' exp1 k'))
  | AwaitE (T.AwaitFut short, exp1) ->
    interpret_exp env exp1
      (fun v1 -> await env exp.at short (V.as_async v1) k)
  | AwaitE (T.AwaitCmp, exp1) ->
    interpret_exp env exp1
      (fun v1 -> V.as_comp v1 k (Option.get env.throws))
  | AssertE (Runtime, exp1) ->
    interpret_exp env exp1 (fun v ->
      if V.as_bool v
      then k V.unit
      else trap exp.at "assertion failure"
    )
  | AnnotE (exp1, _typ) ->
    interpret_exp env exp1 k
  | IgnoreE exp1 ->
    interpret_exp env exp1 (fun _v -> k V.unit)

and interpret_par env par k =
  match par with
  | Some exp -> interpret_exp env exp k
  | None -> V.Obj V.Env.empty |> k

and interpret_exps env exps vs (k : V.value list V.cont) =
  match exps with
  | [] -> k (List.rev vs)
  | exp::exps' ->
    interpret_exp env exp (fun v -> interpret_exps env exps' (v::vs) k)

(* Objects *)

and interpret_exp_fields env exp_fields fld_env (k : V.value V.Env.t V.cont) =
  match exp_fields with
  | [] -> k fld_env
  | exp_field::exp_fields' ->
    interpret_exp env exp_field.it.exp (fun v ->
      let fv = match exp_field.it.mut.it with
          | Syntax.Var -> V.Mut (ref v)
          | Syntax.Const -> v
      in
      interpret_exp_fields env exp_fields' (V.Env.add exp_field.it.id.it fv fld_env) k)

(* Cases *)

and interpret_cases env cases at v (k : V.value V.cont) =
  match cases with
  | [] ->
    trap at "switch value %s does not match any case" (string_of_val env v)
  | {it = {pat; exp}; at; _}::cases' ->
    match match_pat pat v with
    | Some ve -> interpret_exp (adjoin_vals env ve) exp k
    | None -> interpret_cases env cases' at v k

(* Catches *)

and interpret_catches env cases at v (k : V.value V.cont) =
  match cases with
  | [] ->
    Option.get env.throws v (* re-throw v *)
  | {it = {pat; exp}; at; _}::cases' ->
    match match_pat pat v with
    | Some ve -> interpret_exp (adjoin_vals env ve) exp k
    | None -> interpret_catches env cases' at v k

(* Patterns *)

and declare_id id =
  V.Env.singleton id.it (Lib.Promise.make ())

and declare_pat pat : val_env =
  match pat.it with
  | WildP | LitP _ | SignP _ ->  V.Env.empty
  | VarP id -> declare_id id
  | TupP pats -> declare_pats pats V.Env.empty
  | ObjP pfs -> declare_pat_fields pfs V.Env.empty
  | OptP pat1
  | TagP (_, pat1)
  | AltP (pat1, _) (* pat2 has the same identifiers *)
  | AnnotP (pat1, _)
  | ParP pat1 -> declare_pat pat1

and declare_pats pats ve : val_env =
  match pats with
  | [] -> ve
  | pat::pats' ->
    let ve' = declare_pat pat in
    declare_pats pats' (V.Env.adjoin ve ve')

and declare_pat_fields pfs ve : val_env =
  match pfs with
  | [] -> ve
  | pf::pfs' ->
     match pf_pattern pf with
     | Some pat ->
        let ve' = declare_pat pat in
        declare_pat_fields pfs' (V.Env.adjoin ve ve')
     | None ->
        declare_pat_fields pfs' ve

and declare_defined_id id v =
  V.Env.singleton id.it (Lib.Promise.make_fulfilled v)

and define_id env id v =
  define_id' env id.it v

and define_id' env id v =
  Lib.Promise.fulfill (find id env.vals) v

and define_pat env pat v =
  match match_pat pat v with
  | Some ve ->
     V.Env.iter (fun id d  -> define_id' env id (Lib.Promise.value d)) ve;
     true
  | None ->
     false

and match_lit lit v : bool =
  let open Numerics in
  match !lit, v with
  | NullLit, V.Null -> true
  | BoolLit b, V.Bool b' -> b = b'
  | NatLit n, V.Int n' -> Int.eq n n'
  | Nat8Lit n, V.Nat8 n' -> Nat8.eq n n'
  | Nat16Lit n, V.Nat16 n' -> Nat16.eq n n'
  | Nat32Lit n, V.Nat32 n' -> Nat32.eq n n'
  | Nat64Lit n, V.Nat64 n' -> Nat64.eq n n'
  | IntLit i, V.Int i' -> Int.eq i i'
  | Int8Lit i, V.Int8 i' -> Int_8.eq i i'
  | Int16Lit i, V.Int16 i' -> Int_16.eq i i'
  | Int32Lit i, V.Int32 i' -> Int_32.eq i i'
  | Int64Lit i, V.Int64 i' -> Int_64.eq i i'
  | FloatLit z, V.Float z' -> z = z'
  | CharLit c, V.Char c' -> c = c'
  | TextLit u, V.Text u' -> u = u'
  | BlobLit b, V.Blob b' -> b = b'
  | PreLit _, _ -> assert false
  | _ -> false

and match_pat pat v : val_env option =
  match pat.it with
  | WildP -> Some V.Env.empty
  | VarP id -> Some (V.Env.singleton id.it (Lib.Promise.make_fulfilled v))
  | LitP lit ->
    if match_lit lit v
    then Some V.Env.empty
    else None
  | SignP (op, lit) ->
    let t = T.as_immut pat.note in
    match_pat {pat with it = LitP lit} (Operator.unop op t v)
  | TupP pats ->
    match_pats pats (V.as_tup v) V.Env.empty
  | ObjP pfs ->
    match_pat_fields pfs (V.as_obj v) V.Env.empty
  | OptP pat1 ->
    (match v with
    | V.Opt v1 -> match_pat pat1 v1
    | V.Null -> None
    | _ -> assert false
    )
  | TagP (i, pat1) ->
    let k, v1 = V.as_variant v in
    if i.it = k
    then match_pat pat1 v1
    else None
  | AltP (pat1, pat2) ->
    (match match_pat pat1 v with
    | None -> match_pat pat2 v
    | some -> some
    )
  | AnnotP (pat1, _)
  | ParP pat1 ->
    match_pat pat1 v

and match_pats pats vs ve : val_env option =
  match pats, vs with
  | [], [] -> Some ve
  | pat::pats', v::vs' ->
    (match match_pat pat v with
    | Some ve' -> match_pats pats' vs' (V.Env.adjoin ve ve')
    | None -> None
    )
  | _ -> assert false

and match_pat_fields pfs vs ve : val_env option =
  match pfs with
  | [] -> Some ve
  | { it = TypPF(_); _ }::pfs' ->
     match_pat_fields pfs' vs ve
  | { it = ValPF(id, p); _ }::pfs' ->
    let v = V.Env.find id.it vs in
    begin match match_pat p v with
    | Some ve' -> match_pat_fields pfs' vs (V.Env.adjoin ve ve')
    | None -> None
    end

and match_shared_pat env shared_pat c =
  match shared_pat.it, c with
  | T.Local, _ -> V.Env.empty
  | T.Shared (_, pat), v ->
    (match match_pat pat v with
     | None ->
       (* shouldn't occur with our irrefutable patterns, but may in future *)
       trap pat.at "context value %s does not match context pattern" (string_of_val env v)
     | Some ve1 ->
       ve1)

(* Objects *)

and interpret_obj env obj_sort self_id dec_fields (k : V.value V.cont) =
  match obj_sort with
  | T.Actor ->
     let self = V.fresh_id() in
     let self' = V.Blob self in
     (* Define self_id eagerly *)
     let env' = match self_id with
     | Some id -> adjoin_vals env (declare_defined_id id self')
     | None -> env in
     let ve_ex, ve_in = declare_dec_fields dec_fields V.Env.empty V.Env.empty in
     let env'' = adjoin_vals { env' with self } ve_in in
     interpret_dec_fields env'' dec_fields ve_ex
     (fun obj ->
        (env.actor_env := V.Env.add self obj !(env.actor_env);
          k self'))
  | _ ->
     let ve_ex, ve_in = declare_dec_fields dec_fields V.Env.empty V.Env.empty in
     let env' = adjoin_vals env ve_in in
     interpret_dec_fields env' dec_fields ve_ex k

and declare_dec_fields dec_fields ve_ex ve_in : val_env * val_env =
  match dec_fields with
  | [] -> ve_ex, ve_in
  | {it = {dec; vis; _}; _}::dec_fields' ->
    let ve' = declare_dec dec in
    let ve_ex' = if vis.it = Private then ve_ex else V.Env.adjoin ve_ex ve' in
    let ve_in' = V.Env.adjoin ve_in ve' in
    declare_dec_fields dec_fields' ve_ex' ve_in'

and interpret_dec_fields env dec_fields ve (k : V.value V.cont) =
  match dec_fields with
  | [] ->
    let obj = V.Obj (V.Env.map Lib.Promise.value ve) in
    k obj
  | {it = {dec; _}; _}::dec_fields' ->
    interpret_dec env dec (fun _v -> interpret_dec_fields env dec_fields' ve k)


(* Blocks and Declarations *)

and interpret_block env decs ro (k : V.value V.cont) =
  let ve = declare_decs decs V.Env.empty in
  Option.iter (fun r -> r := ve) ro;
  interpret_decs (adjoin_vals env ve) decs k


and declare_dec dec : val_env =
  match dec.it with
  | ExpD _
  | TypD _
  | MixinD (_) -> V.Env.empty
  | IncludeD _ ->
     (* TODO support mixins in the interpreter *)
    assert false
  | LetD (pat, _, _) -> declare_pat pat
  | VarD (id, _) -> declare_id id
  | ClassD (_eo, _, _, id, _, _, _, _, _) -> declare_id {id with note = ()}

and declare_decs decs ve : val_env =
  match decs with
  | [] -> ve
  | dec::decs' ->
    let ve' = declare_dec dec in
    declare_decs decs' (V.Env.adjoin ve ve')


and interpret_dec env dec (k : V.value V.cont) =
  match dec.it with
  | ExpD exp ->
    interpret_exp env exp k
  | LetD (pat, exp, fail) ->
    interpret_exp env exp (fun v ->
      if define_pat env pat v then
        k v
      else
        match fail with
        | Some fail -> interpret_exp env fail (fun _ -> assert false)
        | None -> trap pat.at "value %s does not match pattern" (string_of_val env v)
    )
  | VarD (id, exp) ->
    interpret_exp env exp (fun v ->
      define_id env id (V.Mut (ref v));
      k V.unit
    )
  | TypD _ ->
    k V.unit
  | MixinD _ -> k V.unit
  | IncludeD (_, _arg, _note) ->
     (* TODO
        - evaluate arg and bind it against note.pat
        - define note.imports from mixin as local lets
        - declare/eval note.decs
        - Do we need to recreate the renaming pass?
      *)
     trap dec.at "Mixins are not yet supported in the interpreter"
  | ClassD (_eo, shared_pat, obj_sort, id, _typbinds, pat, _typ_opt, id', dec_fields) ->
    (* NB: we ignore the migration expression _eo *)
    let f = interpret_func env id.it shared_pat pat (fun env' k' ->
      if obj_sort.it <> T.Actor then
        let env'' = adjoin_vals env' (declare_id id') in
        interpret_obj env'' obj_sort.it None dec_fields (fun v' ->
          define_id env'' id' v';
          k' v')
      else
        async env' Source.no_region
          (fun k'' r ->
            let env'' = adjoin_vals env' (declare_id id') in
            let env''' = { env'' with
              labs = V.Env.empty;
              rets = Some k'';
              throws = Some r }
            in
            interpret_obj env''' obj_sort.it (Some id') dec_fields k'')
          k')
    in
    let v = V.Func (CC.call_conv_of_typ dec.note.note_typ, f) in
    define_id env {id with note = ()} v;
    k v

and interpret_decs env decs (k : V.value V.cont) =
  match decs with
  | [] -> k V.unit
  | [dec] -> interpret_dec env dec k
  | dec::decs' ->
    interpret_dec env dec (fun _v -> interpret_decs env decs' k)

and interpret_func env name shared_pat pat f c v (k : V.value V.cont) =
  if env.flags.trace then trace "%s%s" name (string_of_arg env v);
  let v1 = V.Obj (V.Env.singleton "caller" c) in
  let ve1 = match_shared_pat env shared_pat v1 in
  match match_pat pat v with
  | None ->
    trap pat.at "argument value %s does not match parameter list" (string_of_val env v)
  | Some ve2 ->
    incr trace_depth;
    let k' = fun v' ->
      if env.flags.trace then trace "<= %s" (string_of_val env v');
      decr trace_depth;
      k v'
    in
    let env' =
      { env with
        vals = V.Env.adjoin env.vals (V.Env.adjoin ve1 ve2);
        libs = env.libs;
        labs = V.Env.empty;
        rets = Some k';
      }
    in f env' k'

(* Programs *)

let ensure_management_canister env =
  if V.Env.mem "" (!(env.actor_env))
  then ()
  else
    env.actor_env :=
      V.Env.add
        (* ManagementCanister with raw_rand (only) *)
        ""
        (V.Obj
           (V.Env.singleton "raw_rand"
              (V.async_func (T.Write) 0 1
                 (fun c v k ->
                   async env
                     Source.no_region
                     (fun k' r ->
                       k' (V.Blob (V.Blob.rand32 ())))
                     k))))
        !(env.actor_env)

let interpret_prog flags scope p : (V.value * scope) option =
  step_total := 0;
  let state = state in
  try
    let env =
      { (env_of_scope flags state scope) with
        throws = Some (fun v -> trap !last_region "uncaught throw") } in
    ensure_management_canister env;
    trace_depth := 0;
    let vo = ref None in
    let ve = ref V.Env.empty in
    Scheduler.queue (fun () ->
      interpret_block env p.it (Some ve) (fun v -> vo := Some v)
    );
    Scheduler.run ();
    let scope = { val_env = !ve; lib_env = scope.lib_env } in
    match !vo with
    | Some v -> Some (v, scope)
    | None -> None
  with
  | Cancel s ->
    Printf.eprintf "cancelled: %s\n" s;
    None
  | exn ->
    (* For debugging, should never happen. *)
    print_exn flags exn;
    None


(* Libraries *)

(* Import a module unchanged, and a class constructor as an asynchronous function.
   The conversion will be unnecessary once we declare classes as asynchronous. FIXME: is this correct? *)
let import_lib env lib =
  let { body = cub; _ } = lib.it in
  match cub.it with
  | Syntax.ModuleU _ ->
    Fun.id
  | Syntax.ActorClassU (_persistence, _sp, _eo, id, _tbs, _p, _typ, _self_id, _dec_fields) ->
    (* NB: we ignore the migration expression _eo *)
    fun v -> V.Obj (V.Env.from_list
      [ (id.it, v);
        ("system",
         V.Obj (V.Env.singleton id.it (
          V.local_func 1 1 (fun c w k ->
            let tag, w1 = V.as_variant w in
            let o = V.as_obj w1 in
            if tag = "new" && V.Env.find "settings" o = V.Null
            then k v
            else trap cub.at "actor class configuration unsupported in interpreter")))) ])
  | _ -> assert false

let interpret_lib flags scope lib : scope =
  let env = env_of_scope flags state scope in
  trace_depth := 0;
  let vo = ref None in
  let ve = ref V.Env.empty in
  Scheduler.queue (fun () ->
    let import = import_lib env lib in
    let (imp_decs, decs) = CompUnit.decs_of_lib lib in
    interpret_block env (imp_decs @ decs) (Some ve) (fun v ->
      vo := Some (import v))
  );
  Scheduler.run ();
  lib_scope lib.note.filename (Option.get !vo) scope
