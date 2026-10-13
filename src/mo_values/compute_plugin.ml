open Mo_types

type type_parameter = Concrete | Opaque

type arguments =
  | Shared
  | Fixed of (Type.typ list -> (Type.typ list, string) result)

(* Calls are ordinary synchronous local Motoko calls. The continuation retains
   the interpreter's scheduling and actor context; plugins never extract a
   closure and assume that invoking it completes synchronously. *)
type evaluator = {
  call : Type.typ -> Value.value -> Value.value list ->
    ((Value.value, string) result -> unit) -> unit;
}

type specialization = {
  type_arguments : Type.typ list;
  argument_types : Type.typ list;
  result_type : Type.typ;
}

type binding = {
  id : string;
  name : string;
  arguments : arguments;
  type_parameters : type_parameter list;
  result_type : Type.typ list -> Type.typ list -> (Type.typ, string) result;
  evaluate : evaluator -> Type.typ list -> Type.typ list ->
    Value.value list -> ((Value.value, string) result -> unit) -> unit;
}

let pure f _context type_arguments argument_types values k =
  k (f type_arguments argument_types values)

let bindings : binding list ref = ref []

let identifier s =
  String.length s > 0 && s <> "_" &&
  String.for_all (function
    | 'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '_' -> true
    | _ -> false) s &&
  (match s.[0] with '0' .. '9' -> false | _ -> true)

module Prepared = struct
  type t = {owner : string; name : string; con : Type.con}
  let registered : t list ref = ref []

  let create ~owner ~name =
    if owner = "" || not (identifier name) then
      invalid_arg "invalid prepared plugin type identifier";
    {owner; name; con = Cons.fresh name (Type.Abs ([], Type.Any))}

  let typ t = Type.Con (t.con, [])
  let id t = t.owner ^ ":" ^ t.name
  let is_con con = List.exists (fun t -> Cons.eq con t.con) !registered
  let is_registered t = List.exists (fun t' -> t == t') !registered
  let check_registered t =
    if not (is_registered t) then
      invalid_arg ("unregistered prepared plugin type: " ^ t.name)

  let register ~owner t =
    if owner <> t.owner then
      invalid_arg ("prepared plugin type belongs to another plugin: " ^ t.name);
    if List.exists (fun (t' : t) -> t'.name = t.name) !registered ||
       List.exists (fun (b : binding) -> b.name = t.name) !bindings then
      invalid_arg ("duplicate prepared plugin type: " ^ t.name);
    registered := !registered @ [t]

  let check_owner ~owner t =
    check_registered t;
    if owner <> t.owner then
      invalid_arg ("prepared plugin type belongs to another plugin: " ^ t.name)

  let check_type t actual =
    check_registered t;
    if not (Type.sub actual (typ t)) then
      invalid_arg ("prepared plugin value has the wrong type: " ^ t.name)

  let find name =
    Option.map typ (List.find_opt (fun t -> id t = name) !registered)

  let declarations () = List.map (fun (t : t) ->
    Printf.sprintf "type %s = prim %S;" t.name ("compute-type:" ^ id t))
    !registered

  let has_name name = List.exists (fun (t : t) -> t.name = name) !registered
  let wrap t bytes = check_registered t; Value.Blob bytes
  let unwrap t value = check_registered t; Value.as_blob value
end

let register binding =
  if not (identifier binding.name) || not (identifier binding.id) then
    invalid_arg "invalid compute plugin binding identifier";
  if Prepared.has_name binding.name ||
     List.exists (fun b -> b.id = binding.id || b.name = binding.name)
       !bindings then
    invalid_arg ("duplicate compute plugin binding: " ^ binding.name);
  bindings := !bindings @ [binding]

let find id = List.find_opt (fun b -> b.id = id) !bindings

let intrinsic_type id =
  Type.Func (Type.Local, Type.Intrinsic id, [], [], [])

let intrinsic typ = match Type.promote typ with
  | Type.Func (Type.Local, Type.Intrinsic id, [], [], []) ->
    Some id
  | _ -> None

let declarations () =
  String.concat "\n" (Prepared.declarations () @ List.map (fun b ->
    Printf.sprintf "let %s = prim %S;" b.name ("compute:" ^ b.id)) !bindings)

let check_arguments binding type_arguments types =
  match binding.arguments with
  | Shared when List.for_all Type.shared types -> Ok ()
  | Shared -> Error (binding.name ^ " requires shared argument types")
  | Fixed schema ->
    (match schema type_arguments with
     | Error message -> Error message
     | Ok expected when List.length types = List.length expected &&
         List.for_all2 (fun t expected -> Type.sub t expected) types expected -> Ok ()
     | Ok expected -> Error (Printf.sprintf "%s expects arguments of type %s"
         binding.name (Type.string_of_typ (Type.seq expected))))

(* Unlike Type.concrete this also handles incomplete plugin-provided types
   without assertions. Plugins must specialize to a known runtime layout. *)
module Types = Set.Make (Type.Ord)

let resolved typ =
  let seen = ref Types.empty in
  let rec go typ = Types.mem typ !seen || begin
    seen := Types.add typ !seen;
    match typ with
    | Type.Pre -> false
    | Type.Var _ | Type.Prim _ | Type.Any | Type.Non -> true
    | Type.Con (con, ts) ->
      List.for_all go ts &&
      (match Cons.kind con with
       | Type.Abs (bs, _) -> List.length bs = List.length ts
       | Type.Def (bs, body) ->
         List.length bs = List.length ts && go (Type.open_ ts body))
    | Type.Array t | Type.Opt t | Type.Mut t | Type.Weak t
    | Type.Named (_, t) | Type.Async (_, _, t) -> go t
    | Type.Tup ts -> List.for_all go ts
    | Type.Obj (_, fs, _) | Type.Variant fs ->
      List.for_all (fun f -> go f.Type.typ) fs
    | Type.Func (_, _, bs, ins, outs) ->
      List.for_all (fun b -> go b.Type.bound) bs &&
      List.for_all go (ins @ outs)
  end in go typ

let concrete opaque typ =
  let seen = ref Types.empty in
  let rec go typ =
    List.exists (Type.eq typ) opaque || Types.mem typ !seen || begin
      seen := Types.add typ !seen;
      match typ with
      | Type.Var _ | Type.Pre -> false
      | Type.Prim _ | Type.Any | Type.Non -> true
      | Type.Con (con, ts) ->
        (match Cons.kind con with
         | Type.Abs _ -> ts = [] && Prepared.is_con con
         | Type.Def (bs, body) ->
           List.length bs = List.length ts && go (Type.open_ ts body))
      | Type.Array t | Type.Opt t | Type.Mut t | Type.Weak t
      | Type.Named (_, t) -> go t
      | Type.Async (_, _, t) -> go t
      | Type.Tup ts -> List.for_all go ts
      | Type.Obj (_, fs, _) | Type.Variant fs ->
        List.for_all (fun f -> go f.Type.typ) fs
      | Type.Func (_, _, bs, ins, outs) ->
        let ts = Type.open_binds bs in
        List.for_all (fun t -> go (Type.open_ ts t)) (ins @ outs)
    end
  in resolved typ && go typ

let specialize binding type_arguments argument_types =
  let arity = List.length binding.type_parameters in
  if List.length type_arguments <> arity then
    Error (Printf.sprintf "%s expects %d explicit type argument%s"
      binding.name arity (if arity = 1 then "" else "s"))
  else
    let opaque = List.filter_map (fun (mode, typ) ->
      match mode with Opaque -> Some typ | Concrete -> None)
      (List.combine binding.type_parameters type_arguments) in
    if not (List.for_all2 (fun mode typ ->
        if mode = Opaque then resolved typ else concrete [] typ)
        binding.type_parameters type_arguments) then
      Error (binding.name ^ " requires concrete type arguments")
    else if not (List.for_all (concrete opaque) argument_types) then
      Error (binding.name ^ " requires concrete argument types")
    else match check_arguments binding type_arguments argument_types with
    | Error message -> Error message
    | Ok () ->
      match binding.result_type type_arguments argument_types with
      | Error message -> Error message
      | Ok result_type when concrete opaque result_type ->
        Ok {type_arguments; argument_types; result_type}
      | Ok _ -> Error (binding.name ^ " requires a concrete result type")

let map_specialization f s = {
  type_arguments = List.map f s.type_arguments;
  argument_types = List.map f s.argument_types;
  result_type = f s.result_type;
}

let evaluator context = {
  call = (fun typ value arguments k ->
    match Type.promote typ, value with
    | Type.Func (Type.Local, Type.Returns, [], ins, outs),
      Value.Func (cc, f)
      when List.length ins = List.length arguments &&
        cc = Call_conv.local_cc (List.length ins) (List.length outs) ->
      let argument = match arguments with
        | [value] -> value
        | values -> Value.Tup values
      in
      f context argument (fun value -> k (Ok value))
    | _ -> k (Error "compute plugin callback requires a matching synchronous local function"));
}

let evaluate context id s values k = match find id with
  | None -> k (Error ("compute plugin is not loaded: " ^ id))
  | Some binding ->
    (* The frontend checks source annotations and Check_ir checks transformed
       specializations. Only the supplied values vary between executions. *)
    if List.length s.argument_types <> List.length values then
      k (Error (binding.name ^ " received inconsistent argument metadata"))
    else
      binding.evaluate (evaluator context)
        s.type_arguments s.argument_types values k
