module T = Mo_types.Type
module Cons = Mo_types.Cons
module Scope = Mo_frontend.Scope

let magic = "MOI\x01"
let format_version = 1
let compiler_hash = Digest.string Source_id.id

type fingerprint = string

type header = {
  source_hash : fingerprint;
  scope_fingerprint : fingerprint;
  deps : (string * fingerprint) list;
}

(* ---- Binary writer ---- *)

module W = struct
  let create () = Buffer.create 4096
  let contents = Buffer.contents

  let byte buf b = Buffer.add_char buf (Char.chr (b land 0xFF))

  let u32 buf n =
    byte buf n;
    byte buf (n lsr 8);
    byte buf (n lsr 16);
    byte buf (n lsr 24)

  let raw buf s = Buffer.add_string buf s

  let str buf s =
    u32 buf (String.length s);
    raw buf s

  let option buf f = function
    | None -> byte buf 0
    | Some x -> byte buf 1; f buf x

  let list buf f xs =
    u32 buf (List.length xs);
    List.iter (f buf) xs
end

(* ---- Binary reader ---- *)

module R = struct
  type t = { data : string; mutable pos : int }
  let create s = { data = s; pos = 0 }

  let byte r =
    if r.pos >= String.length r.data then
      failwith "moi: unexpected end of data";
    let b = Char.code (String.unsafe_get r.data r.pos) in
    r.pos <- r.pos + 1; b

  let u32 r =
    let b0 = byte r in
    let b1 = byte r in
    let b2 = byte r in
    let b3 = byte r in
    b0 lor (b1 lsl 8) lor (b2 lsl 16) lor (b3 lsl 24)

  let raw r n =
    if r.pos + n > String.length r.data then
      failwith "moi: unexpected end of data";
    let s = String.sub r.data r.pos n in
    r.pos <- r.pos + n; s

  let str r = let n = u32 r in raw r n

  let option r f =
    if byte r = 0 then None else Some (f r)

  let list r f =
    let n = u32 r in
    List.init n (fun _ -> f r)
end

(* ---- Prim / enum tags ---- *)

let prim_tag = function
  | T.Null -> 0  | T.Bool -> 1  | T.Nat -> 2   | T.Nat8 -> 3
  | T.Nat16 -> 4 | T.Nat32 -> 5 | T.Nat64 -> 6 | T.Int -> 7
  | T.Int8 -> 8  | T.Int16 -> 9 | T.Int32 -> 10 | T.Int64 -> 11
  | T.Float -> 12 | T.Float32 -> 13 | T.Char -> 14 | T.Text -> 15
  | T.Blob -> 16 | T.Error -> 17 | T.Principal -> 18 | T.Region -> 19

let tag_prim = function
  | 0 -> T.Null  | 1 -> T.Bool  | 2 -> T.Nat   | 3 -> T.Nat8
  | 4 -> T.Nat16 | 5 -> T.Nat32 | 6 -> T.Nat64 | 7 -> T.Int
  | 8 -> T.Int8  | 9 -> T.Int16 | 10 -> T.Int32 | 11 -> T.Int64
  | 12 -> T.Float | 13 -> T.Float32 | 14 -> T.Char | 15 -> T.Text
  | 16 -> T.Blob | 17 -> T.Error | 18 -> T.Principal | 19 -> T.Region
  | n -> failwith (Printf.sprintf "moi: bad prim tag %d" n)

let obj_sort_tag = function
  | T.Object -> 0 | T.Actor -> 1 | T.Mixin -> 2
  | T.Module -> 3 | T.Memory -> 4

let tag_obj_sort = function
  | 0 -> T.Object | 1 -> T.Actor | 2 -> T.Mixin
  | 3 -> T.Module | 4 -> T.Memory
  | n -> failwith (Printf.sprintf "moi: bad obj_sort tag %d" n)

let func_sort_tag = function
  | T.Local -> 0
  | T.Shared T.Query -> 1
  | T.Shared T.Write -> 2
  | T.Shared T.Composite -> 3

let tag_func_sort = function
  | 0 -> T.Local
  | 1 -> T.Shared T.Query
  | 2 -> T.Shared T.Write
  | 3 -> T.Shared T.Composite
  | n -> failwith (Printf.sprintf "moi: bad func_sort tag %d" n)

let control_tag = function
  | T.Returns -> 0 | T.Promises -> 1 | T.Replies -> 2

let tag_control = function
  | 0 -> T.Returns | 1 -> T.Promises | 2 -> T.Replies
  | n -> failwith (Printf.sprintf "moi: bad control tag %d" n)

let async_sort_tag = function
  | T.Fut -> 0 | T.Cmp -> 1

let tag_async_sort = function
  | 0 -> T.Fut | 1 -> T.Cmp
  | n -> failwith (Printf.sprintf "moi: bad async_sort tag %d" n)

let bind_sort_tag = function
  | T.Scope -> 0 | T.Type -> 1

let tag_bind_sort = function
  | 0 -> T.Scope | 1 -> T.Type
  | n -> failwith (Printf.sprintf "moi: bad bind_sort tag %d" n)

(* ---- Con collection ---- *)

(* Collect all unique cons reachable from a type, in DFS first-encounter order *)
let collect_cons_from_typs typs =
  let seen = ref T.ConSet.empty in
  let acc = ref [] in
  let add_con c =
    if T.ConSet.mem c !seen then false
    else begin seen := T.ConSet.add c !seen; acc := c :: !acc; true end
  in
  let rec walk_typ = function
    | T.Var _ | T.Prim _ | T.Any | T.Non | T.Pre -> ()
    | T.Con (c, ts) ->
      if add_con c then walk_kind (Cons.kind c);
      List.iter walk_typ ts
    | T.Obj (_, fs, tfs) ->
      List.iter walk_field fs;
      List.iter walk_typ_field tfs
    | T.Variant fs -> List.iter walk_field fs
    | T.Array t | T.Opt t | T.Mut t | T.Named (_, t) | T.Weak t ->
      walk_typ t
    | T.Tup ts -> List.iter walk_typ ts
    | T.Func (_, _, bs, ts1, ts2) ->
      List.iter walk_bind bs;
      List.iter walk_typ ts1;
      List.iter walk_typ ts2
    | T.Async (_, s, t) -> walk_typ s; walk_typ t
  and walk_field f = walk_typ f.T.typ
  and walk_typ_field (tf : T.typ_field) =
    if add_con tf.T.typ then walk_kind (Cons.kind tf.T.typ)
  and walk_bind b = walk_typ b.T.bound
  and walk_kind = function
    | T.Def (bs, t) | T.Abs (bs, t) ->
      List.iter walk_bind bs; walk_typ t
  in
  List.iter walk_typ typs;
  List.rev !acc

let collect_cons_from_scope (scope : Scope.t) =
  let typs = T.Env.fold (fun _ t acc -> t :: acc) scope.Scope.lib_env [] in
  collect_cons_from_typs typs

(* ---- Serialize types ---- *)

let write_scope buf (scope : Scope.t) =
  let cons = collect_cons_from_scope scope in
  let con_map =
    List.fold_left (fun (m, i) c -> T.ConEnv.add c i m, i + 1)
      (T.ConEnv.empty, 0) cons |> fst
  in
  let con_index c =
    match T.ConEnv.find_opt c con_map with
    | Some i -> i
    | None -> failwith ("moi: con not in table: " ^ Cons.name c)
  in

  (* write con declarations (names) *)
  W.u32 buf (List.length cons);
  List.iter (fun c -> W.str buf (Cons.name c)) cons;

  (* write con kind bodies *)
  let rec write_typ t =
    match t with
    | T.Var (v, i) -> W.byte buf 0; W.str buf v; W.u32 buf i
    | T.Con (c, ts) ->
      W.byte buf 1; W.u32 buf (con_index c);
      W.u32 buf (List.length ts); List.iter write_typ ts
    | T.Prim p -> W.byte buf 2; W.byte buf (prim_tag p)
    | T.Obj (s, fs, tfs) ->
      W.byte buf 3; W.byte buf (obj_sort_tag s);
      W.list buf write_field fs;
      W.list buf write_typ_field tfs
    | T.Variant fs -> W.byte buf 4; W.list buf write_field fs
    | T.Array t -> W.byte buf 5; write_typ t
    | T.Opt t -> W.byte buf 6; write_typ t
    | T.Tup ts ->
      W.byte buf 7; W.u32 buf (List.length ts); List.iter write_typ ts
    | T.Func (s, c, bs, ts1, ts2) ->
      W.byte buf 8;
      W.byte buf (func_sort_tag s);
      W.byte buf (control_tag c);
      W.list buf write_bind bs;
      W.u32 buf (List.length ts1); List.iter write_typ ts1;
      W.u32 buf (List.length ts2); List.iter write_typ ts2
    | T.Async (s, sc, t) ->
      W.byte buf 9; W.byte buf (async_sort_tag s);
      write_typ sc; write_typ t
    | T.Mut t -> W.byte buf 10; write_typ t
    | T.Any -> W.byte buf 11
    | T.Non -> W.byte buf 12
    | T.Named (n, t) -> W.byte buf 13; W.str buf n; write_typ t
    | T.Weak t -> W.byte buf 14; write_typ t
    | T.Pre -> failwith "moi: cannot serialize Pre"
  and write_field buf (f : T.field) =
    W.str buf f.T.lab;
    write_typ f.T.typ;
    write_src buf f.T.src
  and write_typ_field buf (tf : T.typ_field) =
    W.str buf tf.T.lab;
    W.u32 buf (con_index tf.T.typ);
    write_src buf tf.T.src
  and write_bind buf (b : T.bind) =
    W.str buf b.T.var;
    W.byte buf (bind_sort_tag b.T.sort);
    write_typ b.T.bound
  and write_src buf (src : T.src) =
    W.option buf (fun buf s -> W.str buf s) src.T.depr
  in

  let write_kind = function
    | T.Def (bs, t) ->
      W.byte buf 0; W.list buf write_bind bs; write_typ t
    | T.Abs (bs, t) ->
      W.byte buf 1; W.list buf write_bind bs; write_typ t
  in
  List.iter (fun c -> write_kind (Cons.kind c)) cons;

  (* write lib_env *)
  let lib_entries = T.Env.bindings scope.Scope.lib_env in
  W.u32 buf (List.length lib_entries);
  List.iter (fun (key, typ) ->
    W.str buf key;
    write_typ typ
  ) lib_entries

(* ---- Deserialize types ---- *)

let read_scope r =
  let num_cons = R.u32 r in
  let con_names = Array.init num_cons (fun _ -> R.str r) in

  (* pre-create all cons with Pre placeholder *)
  let cons = Array.init num_cons (fun i ->
    Cons.fresh con_names.(i) (T.Abs ([], T.Pre))
  ) in

  let con_lookup i =
    if i < 0 || i >= num_cons then
      failwith (Printf.sprintf "moi: con index %d out of range [0,%d)" i num_cons);
    cons.(i)
  in

  let rec read_typ () =
    match R.byte r with
    | 0 -> let v = R.str r in let i = R.u32 r in T.Var (v, i)
    | 1 ->
      let ci = R.u32 r in
      let n = R.u32 r in
      let ts = List.init n (fun _ -> read_typ ()) in
      T.Con (con_lookup ci, ts)
    | 2 -> T.Prim (tag_prim (R.byte r))
    | 3 ->
      let s = tag_obj_sort (R.byte r) in
      let fs = R.list r read_field in
      let tfs = R.list r read_typ_field in
      T.Obj (s, fs, tfs)
    | 4 -> T.Variant (R.list r read_field)
    | 5 -> T.Array (read_typ ())
    | 6 -> T.Opt (read_typ ())
    | 7 -> let n = R.u32 r in T.Tup (List.init n (fun _ -> read_typ ()))
    | 8 ->
      let s = tag_func_sort (R.byte r) in
      let c = tag_control (R.byte r) in
      let bs = R.list r read_bind in
      let n1 = R.u32 r in
      let ts1 = List.init n1 (fun _ -> read_typ ()) in
      let n2 = R.u32 r in
      let ts2 = List.init n2 (fun _ -> read_typ ()) in
      T.Func (s, c, bs, ts1, ts2)
    | 9 ->
      let s = tag_async_sort (R.byte r) in
      let sc = read_typ () in
      let t = read_typ () in
      T.Async (s, sc, t)
    | 10 -> T.Mut (read_typ ())
    | 11 -> T.Any
    | 12 -> T.Non
    | 13 -> let n = R.str r in T.Named (n, read_typ ())
    | 14 -> T.Weak (read_typ ())
    | tag -> failwith (Printf.sprintf "moi: bad typ tag %d" tag)
  and read_field r =
    let lab = R.str r in
    let typ = read_typ () in
    let src = read_src () in
    { T.lab; typ; src }
  and read_typ_field r =
    let lab = R.str r in
    let ci = R.u32 r in
    let src = read_src () in
    { T.lab; typ = con_lookup ci; src }
  and read_bind r =
    let var = R.str r in
    let sort = tag_bind_sort (R.byte r) in
    let bound = read_typ () in
    { T.var; sort; bound }
  and read_src () =
    let depr = R.option r (fun r -> R.str r) in
    { T.depr;
      track_region = Source.no_region;
      region = Source.no_region }
  in

  (* read con kinds and set them *)
  let read_kind () =
    match R.byte r with
    | 0 ->
      let bs = R.list r read_bind in
      let t = read_typ () in
      T.Def (bs, t)
    | 1 ->
      let bs = R.list r read_bind in
      let t = read_typ () in
      T.Abs (bs, t)
    | tag -> failwith (Printf.sprintf "moi: bad kind tag %d" tag)
  in
  for i = 0 to num_cons - 1 do
    let k = read_kind () in
    T.set_kind cons.(i) k
  done;

  (* read lib_env *)
  let num_libs = R.u32 r in
  let lib_env = ref T.Env.empty in
  for _ = 1 to num_libs do
    let key = R.str r in
    let typ = read_typ () in
    lib_env := T.Env.add key typ !lib_env
  done;
  { Scope.empty with Scope.lib_env = !lib_env }

(* ---- Fingerprinting ---- *)

let hash_file path =
  Digest.file path

let compute_fingerprint ~source_hash ~deps =
  let sorted = List.sort (fun (a, _) (b, _) -> String.compare a b) deps in
  let buf = Buffer.create 256 in
  Buffer.add_string buf source_hash;
  List.iter (fun (name, fp) ->
    Buffer.add_string buf name;
    Buffer.add_string buf fp
  ) sorted;
  Digest.string (Buffer.contents buf)

(* ---- .moi file path ---- *)

let moi_path ~cache_dir ~import_key =
  let hash = Digest.string import_key in
  let hex = Digest.to_hex hash in
  let base = Filename.basename import_key in
  let base = try Filename.chop_extension base with Invalid_argument _ -> base in
  Filename.concat cache_dir (Printf.sprintf "%s-%s.moi" (String.sub hex 0 8) base)

(* ---- .moi file I/O ---- *)

let rec mkdir_p dir =
  if Sys.file_exists dir then ()
  else begin
    mkdir_p (Filename.dirname dir);
    (try Sys.mkdir dir 0o755 with Sys_error _ -> ())
  end

let write_moi_file path header scope =
  let buf = W.create () in
  W.raw buf magic;
  W.u32 buf format_version;
  W.raw buf compiler_hash;
  W.raw buf header.source_hash;
  W.raw buf header.scope_fingerprint;
  W.list buf (fun buf (name, fp) -> W.str buf name; W.raw buf fp) header.deps;
  write_scope buf scope;
  let data = W.contents buf in
  mkdir_p (Filename.dirname path);
  let tmp = path ^ ".tmp" in
  let oc = open_out_bin tmp in
  Fun.protect ~finally:(fun () -> close_out oc) (fun () ->
    output_string oc data
  );
  Sys.rename tmp path

let digest_len = 16  (* MD5 *)

let read_moi_file path =
  if not (Sys.file_exists path) then None
  else
    try
      let ic = open_in_bin path in
      let data = Fun.protect ~finally:(fun () -> close_in ic) (fun () ->
        In_channel.input_all ic
      ) in
      let r = R.create data in
      let file_magic = R.raw r 4 in
      if file_magic <> magic then None
      else
        let ver = R.u32 r in
        if ver <> format_version then None
        else
        let stored_compiler_hash = R.raw r digest_len in
        if stored_compiler_hash <> compiler_hash then None
        else begin
          let source_hash = R.raw r digest_len in
          let scope_fingerprint = R.raw r digest_len in
          let deps = R.list r (fun r ->
            let name = R.str r in
            let fp = R.raw r digest_len in
            (name, fp)
          ) in
          let header = { source_hash; scope_fingerprint; deps } in
          let scope = read_scope r in
          Some (header, scope)
        end
    with
    | Failure _ | Sys_error _ | Invalid_argument _ -> None
    | exn ->
      Printf.eprintf "moi: warning: unexpected error reading %s: %s\n"
        path (Printexc.to_string exn);
      None

(* ---- Public API ---- *)

let load ~cache_dir ~source_path ~source_hash ~dep_fingerprints =
  let path = moi_path ~cache_dir ~import_key:source_path in
  match read_moi_file path with
  | None -> None
  | Some (header, scope) ->
    if header.source_hash <> source_hash then
      None
    else
      let deps_valid = List.for_all (fun (dep_name, stored_fp) ->
        match dep_fingerprints dep_name with
        | Some current_fp -> current_fp = stored_fp
        | None -> false
      ) header.deps in
      if not deps_valid then None
      else Some (header, scope)

let save ~cache_dir ~source_path ~header ~scope =
  let path = moi_path ~cache_dir ~import_key:source_path in
  try write_moi_file path header scope
  with exn ->
    Printf.eprintf "moi: warning: failed to write cache %s: %s\n"
      path (Printexc.to_string exn)
