(* 
This module contains common definitions for both compile_classical.ml and compile_enhanced.ml.
*)
open Mo_types

open Wasm_exts.Ast
open Wasm_exts.Types

(* Re-shadow Source.(@@), to get Stdlib.(@@) *)
let (@@) = Stdlib.(@@)

module G = InstrList
let (^^) = G.(^^) (* is this how we import a single operator from a module that we otherwise use qualified? *)

(* Our code depends on OCaml int having at least 32 bits *)
let _ = assert (Sys.int_size >= 32)

(* Generating function names for functions parametrized by prim types *)
let prim_fun_name p stem = Printf.sprintf "%s<%s>" stem (Type.string_of_prim p)

(* Helper functions to produce annotated terms (Wasm.AST) *)
let nr x = Wasm.Source.{ it = x; at = no_region }

let todo fn se x = Printf.eprintf "%s: %s" fn (Wasm.Sexpr.to_string 80 se); x

exception CodegenError of string
let fatal fmt = Printf.ksprintf (fun s -> raise (CodegenError s)) fmt

(* Table used for fast adding to the end of the sequence *)
module Table : sig
  type 'a t
  val empty : 'a t
  val add : 'a t -> 'a -> int * 'a t
  val length : 'a t -> int
  val to_list : 'a t -> 'a list
  val from_list : 'a list ->'a t
end = struct
  type 'a t = int * 'a list
  let empty = (0, [])
  let add (l, es) e = (l, (l + 1, e :: es))
  let length (l, es) = l
  let to_list (l, es) = List.rev es
  let from_list es = (List.length es, List.rev es)
end

(* Common definitions for the Compiler Environment *)
module E = struct

  (* Utilities, internal to E *)
  let reg (ref : 'a Table.t ref) (x : 'a) : int32 =
      let i, t = Table.add !ref x in
      ref := t;
      Wasm.I32.of_int_u i

  let reserve_promise (ref : 'a Lib.Promise.t Table.t ref) _s : (int32 * ('a -> unit)) =
    let p = Lib.Promise.make () in (* For debugging with named promises, use s here *)
    let (i, t) = Table.add !ref p in
    let i32 = Wasm.I32.of_int_u i in
    ref := t;
    (i32, Lib.Promise.fulfill p)


  module NameEnv = Env.Make(String)
  
  type local_names = (int32 * string) list (* For the debug section: Names of locals *)
  type func_with_names = func * local_names
  type lazy_function = (int32, func_with_names) Lib.AllocOnUse.t
end

(* Module for managing imports *)
module Imports = struct
  open E

  type t = {
    func_types : func_type Table.t ref;
    potential_func_imports : (import * bool ref) Table.t ref;
    funcs : (func * string * local_names) Lib.Promise.t Table.t ref;
    named_imports : (int32 * bool ref) NameEnv.t ref;
  }

  let empty () = {
    func_types = ref Table.empty;
    potential_func_imports = ref Table.empty;
    funcs = ref Table.empty;
    named_imports = ref NameEnv.empty;
  }

  let reserve_fun (env : t) name =
    let (j, fill) = reserve_promise env.funcs name in
    let n = Int32.of_int (Table.length !(env.potential_func_imports)) in
    let fi = Int32.add j n in
    let fill_ (f, local_names) = fill (f, name, local_names) in
    (fi, fill_)

  let add_fun env name (f, local_names) =
    let (fi, fill) = reserve_fun env name in
    fill (f, local_names);
    fi

  let get_funcs (env : t) = List.map Lib.Promise.value (Table.to_list !(env.funcs))

  let func_type env ty =
    let rec go i = function
      | [] ->
         let (i, t) = Table.add !(env.func_types) ty in
         env.func_types := t;
         Int32.of_int i
      | ty'::tys when ty = ty' -> Int32.of_int i
      | _ :: tys -> go (i+1) tys
       in
    go 0 (Table.to_list !(env.func_types))

  let get_types env = Table.to_list !(env.func_types)

  let add_func_import (env : t) modname funcname arg_tys ret_tys =
    if Table.length !(env.funcs) <> 0 then
      raise (CodegenError "Add all imports before all functions!");

    let i = {
      module_name = Lib.Utf8.decode modname;
      item_name = Lib.Utf8.decode funcname;
      idesc = nr (FuncImport (nr (func_type env (FuncType (arg_tys, ret_tys)))))
    } in
    let used = ref false in
    let fi = reg env.potential_func_imports (nr i, used) in
    let name = modname ^ "." ^ funcname in
    assert (not (NameEnv.mem name !(env.named_imports)));
    env.named_imports := NameEnv.add name (fi, used) !(env.named_imports)

  let reuse_import env modname funcname =
    let name = modname ^ "." ^ funcname in
    match NameEnv.find_opt name !(env.named_imports) with
      | Some (fi, used) ->
        used := true;
        fi
      | _ ->
        raise (Invalid_argument (Printf.sprintf "Function import not declared: %s\n" name))

  let call_import env modname funcname =
    let fi = reuse_import env modname funcname in
    G.i (Call (nr fi))

  let finalize_func_imports env : import list * int32 * (int32 -> int32) =
    let module M = Map.Make(struct type t = int32 let compare = Int32.compare end) in
    (* Skip unused imports and build the remapping for the used ones *)
    let func_imports, ni', import_remap =
      let i' = ref 0 in
      let remap = ref M.empty in
      let rec go i acc = function
        | [] -> 
          assert (i = Table.length !(env.potential_func_imports));
          List.rev acc
        | (imp, used) :: imps ->
          if !used then begin
            remap := M.add (Int32.of_int i) (Int32.of_int !i') !remap;
            i' := !i' + 1;
            go (i + 1) (imp :: acc) imps
          end else
            go (i + 1) acc imps
      in
      let imports = go 0 [] (Table.to_list !(env.potential_func_imports)) in
      assert (!i' = List.length imports);
      imports, Int32.of_int !i', !remap
    in
    let remapping =
      let old_num_imports = Table.length !(env.potential_func_imports) |> Int32.of_int in
      let offset = Int32.sub old_num_imports ni' in
      fun old_index ->
        if old_index < old_num_imports then
          (* It's an import. Find its new index in the map. *)
          (* This should raise an exception if a call to an unused import is found. *)
          M.find old_index import_remap
        else
          (* It's a module-defined function. Adjust its index. *)
          Int32.sub old_index offset
    in
    func_imports, ni', remapping
end

module Make_Ptr_Logic (S : sig
  type t
  val ty : value_type
  val align : int
  val skew : t
  val unskew : t
end) = struct
  (*
  Pointers are skewed (translated) -1 relative to the actual offset.
  See documentation of module BitTagged for more detail.
  *)
  let ptr_skew = S.skew
  let ptr_unskew = S.unskew
  (* Pointer reference and dereference  *)
  let load_unskewed_ptr = G.i (Load {ty = S.ty; align = S.align; offset = 0L; sz = None})
  let store_unskewed_ptr = G.i (Store {ty = S.ty; align = S.align; offset = 0L; sz = None})
  let load_ptr = G.i (Load {ty = S.ty; align = S.align; offset = 1L; sz = None})
  let store_ptr = G.i (Store {ty = S.ty; align = S.align; offset = 1L; sz = None})
end

module W32_Pointers = Make_Ptr_Logic (struct
  type t = int32
  let (ty, align, skew, unskew) = (I32Type, 2, -1l, 1l)
end)

module W64_Pointers = Make_Ptr_Logic (struct
  type t = int64
  let (ty, align, skew, unskew) = (I64Type, 3, -1L, 1L)
end)
