open Source
open Syntax
open Wasm.Sexpr

let string_of_prim p =
  match p with
  | Nat -> "nat"
  | Nat8 -> "nat8"
  | Nat16 -> "nat16"
  | Nat32 -> "nat32"
  | Nat64 -> "nat64"
  | Int -> "int"
  | Int8 -> "int8"
  | Int16 -> "int16"
  | Int32 -> "int32"
  | Int64 -> "int64"
  | Float32 -> "float32"
  | Float64 -> "float64"
  | Bool -> "bool"
  | Text -> "text"
  | Null -> "null"
  | Reserved -> "reserved"
  | Empty -> "empty"

let string_of_mode m =
  match m.it with
  | Oneway -> " oneway"
  | Query -> " query"
  | Composite -> " composite_query"


let ($$) head inner = Node (head, inner)

and id i = Atom i.it
and tag i = Atom ("#" ^ i.it)

let field_tag (tf : field_label)
  = match tf.it with
  | Id n -> Lib.Uint32.to_string n
  | Named name -> name
  | Unnamed n -> Lib.Uint32.to_string n

let rec typ_field (tf : typ_field)
  = field_tag (tf.it.label) $$ [typ tf.it.typ]

and typ_meth (tb : typ_meth)
  = tb.it.var.it $$ [typ tb.it.meth]

and mode m = Atom (string_of_mode m)

and typ t = match t.it with
  | VarT s           -> "VarT" $$ [id s]
  | PrimT p          -> "PrimT" $$ [Atom (string_of_prim p)]
  | RecordT ts       -> "RecordT" $$ List.map typ_field ts
  | VecT t           -> "VecT" $$ [typ t]
  | BlobT            -> Atom "BlobT"
  | OptT t           -> "OptT" $$ [typ t]
  | VariantT cts     -> "VariantT" $$ List.map typ_field cts
  | FuncT (ms, s, t) -> "FuncT" $$ List.map arg_typ s @ List.map arg_typ t @ List.map mode ms
  | ServT ts         -> "ServT" $$ List.map typ_meth ts
  | ClassT (ts, t)   -> "ClassT" $$ List.map arg_typ ts @ [typ t]
  | PrincipalT       -> Atom "PrincipalT"
  | PreT             -> Atom "PreT"

and arg_typ t =
  match t.it.name with
  | Some name -> "Named" $$ [Atom name.it; typ t.it.typ]
  | None ->  typ t.it.typ

and dec d = match d.it with
  | TypD (x, t) ->
     "TypD" $$ [id x] @ [typ t]
  | ImportD (f, fp) ->
     "ImportD" $$ [Atom (if !fp = "" then f else !fp)]

and actor = function
  | None -> Atom "NoActor"
  | Some t ->
     "Actor" $$ [typ t]

and prog prog = "Decs" $$ List.map dec prog.it.decs @ [actor prog.it.actor]


(* Pretty printing  *)
module type Config = sig
  val trivia : Trivia.trivia_info Trivia.PosHashtbl.t option
end

module Default = struct
  let trivia = None
end

module Make (Cfg : Config) = struct
  open Format
  let str ppf s = pp_print_string ppf s
  let space = pp_print_space
  let kwd ppf s = str ppf s; space ppf ()
  let quote ppf s =
    pp_open_hbox ppf ();
    str ppf "\""; str ppf (Lib.String.lightweight_escaped s); str ppf "\"";
    pp_close_box ppf ()
  let text ppf s =
    if Escape.needs_candid_quote s then quote ppf s else str ppf s

  let rec pp_typ ppf t =
    pp_open_hovbox ppf 1;
    (match t.it with
    | VarT id -> str ppf id.it
    | PrimT p -> str ppf (string_of_prim p)
    | OptT t -> kwd ppf "opt"; pp_typ ppf t
    | VecT t -> kwd ppf "vec"; pp_typ ppf t
    | BlobT -> str ppf "blob"
    | RecordT fs -> pp_fields ppf "record" fs
    | VariantT fs -> pp_fields ppf "variant" fs
    | FuncT (ms,s,t) ->
      kwd ppf "func";
      pp_func ppf (ms,s,t)
    | ServT ms ->
      pp_open_vbox ppf 2;
      str ppf "service {";
      List.iter (fun m -> pp_print_cut ppf (); pp_meth ppf m; str ppf ";") ms;
      pp_print_break ppf 0 (-2);
      str ppf "}";
      pp_close_box ppf ()
    | PrincipalT -> str ppf "principal"
    | ClassT _ -> assert false
    | PreT -> assert false);
    pp_close_box ppf ()
  and pp_fields ppf name fs =
    let is_variant = name = "variant" in
    if List.length fs > 1 then
      pp_open_vbox ppf 2
    else
      pp_open_hovbox ppf 2;
    str ppf (name ^ " {");
    List.iter (fun f -> pp_print_cut ppf (); pp_field ppf is_variant f; str ppf ";") fs;
    pp_print_break ppf 0 (-2);
    str ppf "}";
    pp_close_box ppf ()
  and pp_field ppf is_variant f =
    pp_doc ppf f.at;
    let hide_type = is_variant && f.it.typ.it = PrimT Null in
    pp_open_hovbox ppf 1;
    (match f.it.label.it with
    | Named name ->
      text ppf name;
      if not hide_type then
        (kwd ppf ":"; pp_typ ppf f.it.typ)
    | Id n ->
      str ppf (Lib.Uint32.to_string n);
      if not hide_type then
        (kwd ppf ":"; pp_typ ppf f.it.typ)
    | Unnamed _ -> pp_typ ppf f.it.typ);
    pp_close_box ppf ()

  and pp_func ppf (ms,s,t) =
    pp_args ppf s;
    kwd ppf " ->";
    pp_args ppf t;
    List.iter (fun m -> str ppf (string_of_mode m)) ms

  and pp_args ppf fs =
    let n = List.length fs in
    str ppf "(";
    List.iteri (fun i f ->
        (match f.it.name with
          Some name -> (text ppf name.it; kwd ppf ":")
        | None -> ());
        pp_typ ppf f.it.typ;
        if i < n-1 then
          kwd ppf ",";
      ) fs;
    str ppf ")"

  and pp_meth ppf m =
    pp_doc ppf m.at;
    pp_open_hovbox ppf 1;
    text ppf m.it.var.it;
    kwd ppf ":";
    (match m.it.meth.it with
    | FuncT (ms,s,t) -> pp_func ppf (ms,s,t)
    | _ -> pp_typ ppf m.it.meth);
    pp_close_box ppf ()

  and pp_doc ppf at =
    let open Trivia in
    match Cfg.trivia with
    | Some t ->
      let pos = { line = at.left.line; column = at.left.column } in
      let trivia = PosHashtbl.find_opt t pos in
      (match trivia with
      | Some t ->
        List.iter (function
        | LineComment s ->
          str ppf "/// ";
          str ppf s;
          pp_force_newline ppf ()
        | BlockComment s ->
          List.iter (fun line ->
            if String.length line > 0 then (
              str ppf "/// ";
              str ppf line;
              pp_force_newline ppf ()))
          (String.split_on_char '\n' s))
        (docs_of_trivia_info t)
      | None -> ())
    | None -> ()

  let rec is_linebreak_type t =
    match t.it with
    | ServT _ -> true
    | RecordT fs | VariantT fs -> List.length fs > 1
    | VecT t | OptT t -> is_linebreak_type t
    | _ -> false

  let pp_dec ppf d =
    pp_doc ppf d.at;
    pp_open_vbox ppf 1;
    (match d.it with
    | TypD (id, typ) ->
        pp_open_hbox ppf ();
        kwd ppf "type";
        kwd ppf id.it;
        kwd ppf "=";
        pp_close_box ppf ();
        if is_linebreak_type typ then
          pp_print_cut ppf ();
        pp_typ ppf typ
    | ImportD (f, fp) ->
        str ppf "import \"";
        str ppf f;
        str ppf "\""
    );
    pp_close_box ppf ()

  let pp_actor ppf actor =
    Option.iter (fun a -> pp_doc ppf a.at) actor;
    (match actor with
    | None -> ()
    | Some {it=ServT ms; _} ->
      pp_open_vbox ppf 2;
      pp_open_hbox ppf ();
      str ppf "service : {";
      pp_close_box ppf ();
      List.iter (fun m -> pp_print_cut ppf (); pp_meth ppf m; str ppf ";") ms;
      pp_print_break ppf 0 (-2);
      str ppf "}";
      pp_close_box ppf ()
    | Some {it=VarT x; _} ->
      pp_open_hbox ppf ();
      kwd ppf "service";
      kwd ppf ":";
      str ppf x.it;
      pp_close_box ppf ()
    | Some {it=ClassT(args, {it = ServT ms;_ }); _} ->
      pp_open_hbox ppf ();
      kwd ppf "service";
      kwd ppf ":";
      pp_args ppf args;
      str ppf " -> {";
      pp_open_vbox ppf 2;
      List.iter (fun m -> pp_print_cut ppf (); pp_meth ppf m; str ppf ";") ms;
      pp_print_break ppf 0 (-2);
      str ppf "}";
      pp_close_box ppf ();
      pp_close_box ppf ()
    | Some {it=ClassT(args, t); _} ->
      pp_open_hbox ppf ();
      kwd ppf "service";
      kwd ppf ":";
      pp_args ppf args;
      str ppf " -> ";
      pp_typ ppf t;
      pp_close_box ppf ()
    | _ -> assert false);
    pp_print_cut ppf ()

  let pp_prog ppf prog =
    pp_open_vbox ppf 0;
    List.iter (fun d ->
        pp_dec ppf d;
        str ppf ";";
        pp_print_cut ppf ()
      ) prog.it.decs;
    pp_doc ppf prog.at;
    pp_actor ppf prog.it.actor;
    pp_close_box ppf ()

  let string_of_typ t =
    let buf = Buffer.create 100 in
    let ppf = formatter_of_buffer buf in
    pp_typ ppf t;
    pp_print_flush ppf ();
    Buffer.contents buf

  let string_of_prog prog =
    let buf = Buffer.create 100 in
    let ppf = formatter_of_buffer buf in
    pp_prog ppf prog;
    pp_print_flush ppf ();
    Buffer.contents buf

  let string_of_args ts =
    let buf = Buffer.create 100 in
    let ppf = formatter_of_buffer buf in
    pp_args ppf ts;
    pp_print_flush ppf ();
    Buffer.contents buf
end

(* Defaults *)
include Make (Default)
