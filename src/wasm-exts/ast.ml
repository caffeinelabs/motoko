(*
This module originated as a copy of interpreter/syntax/ast.ml in the
reference implementation.

Base revision: WebAssembly/spec@a7a1856.

The changes are:
 * Manual selective support for bulk-memory operations `memory_copy` and `memory_fill` (WebAssembly/spec@7fa2f20).
 * Pseudo-instruction Meta for debug information
 * StableMemory, StableGrow, StableRead, StableWrite instructions.
 * Support for passive data segments (incl. `MemoryInit`).
 * Support for table index in `call_indirect` (reference-types proposal).

The code is otherwise as untouched as possible, so that we can relatively
easily apply diffs from the original code (possibly manually).
*)

(*
 * Throughout the implementation we use consistent naming conventions for
 * syntactic elements, associated with the types defined here and in a few
 * other places:
 *
 *   x : var
 *   v : value
 *   e : instr
 *   f : func
 *   m : module_
 *
 *   t : value_type
 *   s : func_type
 *   c : context / config
 *
 * These conventions mostly follow standard practice in language semantics.
 *)

open Types
open Wasm.Source


(* Operators *)

module IntOp =
struct
  type unop = Clz | Ctz | Popcnt | ExtendS of pack_size
  type binop = Add | Sub | Mul | DivS | DivU | RemS | RemU
             | And | Or | Xor | Shl | ShrS | ShrU | Rotl | Rotr
  type testop = Eqz
  type relop = Eq | Ne | LtS | LtU | GtS | GtU | LeS | LeU | GeS | GeU
  type cvtop = ExtendSI32 | ExtendUI32 | WrapI64
             | TruncSF32 | TruncUF32 | TruncSF64 | TruncUF64
             | TruncSatSF32 | TruncSatUF32 | TruncSatSF64 | TruncSatUF64
             | ReinterpretFloat
end

module FloatOp =
struct
  type unop = Neg | Abs | Ceil | Floor | Trunc | Nearest | Sqrt
  type binop = Add | Sub | Mul | Div | Min | Max | CopySign
  type testop
  type relop = Eq | Ne | Lt | Gt | Le | Ge
  type cvtop = ConvertSI32 | ConvertUI32 | ConvertSI64 | ConvertUI64
             | PromoteF32 | DemoteF64
             | ReinterpretInt
end

module I32Op = IntOp
module I64Op = IntOp
module F32Op = FloatOp
module F64Op = FloatOp

type unop = (I32Op.unop, I64Op.unop, F32Op.unop, F64Op.unop) Values.op
type binop = (I32Op.binop, I64Op.binop, F32Op.binop, F64Op.binop) Values.op
type testop = (I32Op.testop, I64Op.testop, F32Op.testop, F64Op.testop) Values.op
type relop = (I32Op.relop, I64Op.relop, F32Op.relop, F64Op.relop) Values.op
type cvtop = (I32Op.cvtop, I64Op.cvtop, F32Op.cvtop, F64Op.cvtop) Values.op

type 'a memop =
  {ty : value_type; align : int; offset : Memory.offset; sz : 'a option}
type loadop = (pack_size * extension) memop
type storeop = pack_size memop


(* Expressions *)

type var = int32 phrase
type literal = Values.value phrase
type name = int list

type block_type = VarBlockType of var | ValBlockType of value_type option

type instr = instr' phrase
and instr' =
  | Unreachable                       (* trap unconditionally *)
  | Nop                               (* do nothing *)
  | Drop                              (* forget a value *)
  | Select                            (* branchless conditional *)
  | Block of block_type * instr list  (* execute in sequence *)
  | Loop of block_type * instr list   (* loop header *)
  | If of block_type * instr list * instr list  (* conditional *)
  | Br of var                         (* break to n-th surrounding label *)
  | BrIf of var                       (* conditional break *)
  | BrTable of var list * var         (* indexed break *)
  | Return                            (* break from function body *)
  | Call of var                       (* call function *)
  | CallIndirect of var * var         (* call function through table *)
  | LocalGet of var                   (* read local variable *)
  | LocalSet of var                   (* write local variable *)
  | LocalTee of var                   (* write local variable and keep value *)
  | GlobalGet of var                  (* read global variable *)
  | GlobalSet of var                  (* write global variable *)
  | Load of loadop                    (* read memory at address *)
  | Store of storeop                  (* write memory at address *)
  | MemorySize                        (* size of linear memory *)
  | MemoryGrow                        (* grow linear memory *)
  (* Manual extension for bulk memory operations *)
  | MemoryFill                        (* fill memory range with value *)
  | MemoryCopy                        (* copy memory ranges *)
  (* End of manual extension *)
  (* Manual extension for passive data segments *)
  | MemoryInit of var                 (* initialize memory range from segment *)
  (* End of manual extension *)
  | Const of literal                  (* constant *)
  | Test of testop                    (* numeric test *)
  | Compare of relop                  (* numeric comparison *)
  | Unary of unop                     (* unary numeric operator *)
  | Binary of binop                   (* binary numeric operator *)
  | Convert of cvtop                  (* conversion *)

  (* Custom addition for debugging *)
  | Meta of Dwarf5.Meta.die           (* debugging metadata *)

  (* Custom additions for emulating stable-memory, special cases
     of MemorySize, MemoryGrow and MemoryCopy
     requiring wasm features bulk-memory and multi-memory
  *)
  | StableSize                        (* size of stable memory *)
  | StableGrow                        (* grow stable memory *)
  | StableRead                        (* read from stable memory *)
  | StableWrite                       (* write to stable memory *)

(* Globals & Functions *)

type const = instr list phrase

type global = global' phrase
and global' =
{
  gtype : global_type;
  value : const;
}

type func = func' phrase
and func' =
{
  ftype : var;
  locals : value_type list;
  body : instr list;
}


(* Tables & Memories *)

type table = table' phrase
and table' =
{
  ttype : table_type;
}

type memory = memory' phrase
and memory' =
{
  mtype : memory_type;
}

type 'data segment = 'data segment' phrase
and 'data segment' =
{
  index : var;
  offset : const;
  init : 'data;
}

type table_segment = var list segment

(* Manual extension to support passive data segements *)
type segment_mode = segment_mode' phrase
and segment_mode' =
  | Passive
  | Active of {index : var; offset : const}
  | Declarative

type data_segment = data_segment' phrase
and data_segment' =
{
  dinit : string;
  dmode : segment_mode;
}
(* End of manual extension *)

(* Modules *)

type type_ = func_type phrase

type export_desc = export_desc' phrase
and export_desc' =
  | FuncExport of var
  | TableExport of var
  | MemoryExport of var
  | GlobalExport of var

type export = export' phrase
and export' =
{
  name : name;
  edesc : export_desc;
}

type import_desc = import_desc' phrase
and import_desc' =
  | FuncImport of var
  | TableImport of table_type
  | MemoryImport of memory_type
  | GlobalImport of global_type

type import = import' phrase
and import' =
{
  module_name : name;
  item_name : name;
  idesc : import_desc;
}

type module_ = module_' phrase
and module_' =
{
  types : type_ list;
  globals : global list;
  tables : table list;
  memories : memory list;
  funcs : func list;
  start : var option;
  elems : var list segment list;
  (* Manual adjustment for passive data segment support *)
  datas : data_segment list;
  (* End of manual adjustment *)
  imports : import list;
  exports : export list;
}


(* Auxiliary functions *)

let empty_module =
{
  types = [];
  globals = [];
  tables = [];
  memories = [];
  funcs = [];
  start = None;
  elems  = [];
  datas = [];
  imports = [];
  exports = [];
}


let func_type_for (m : module_) (x : var) : func_type =
  (Lib.List32.nth m.it.types x.it).it

let import_type (m : module_) (im : import) : extern_type =
  let {idesc; _} = im.it in
  match idesc.it with
  | FuncImport x -> ExternFuncType (func_type_for m x)
  | TableImport t -> ExternTableType t
  | MemoryImport t -> ExternMemoryType t
  | GlobalImport t -> ExternGlobalType t

let export_type (m : module_) (ex : export) : extern_type =
  let {edesc; _} = ex.it in
  let its = List.map (import_type m) m.it.imports in
  let open Lib.List32 in
  match edesc.it with
  | FuncExport x ->
    let fts =
      funcs its @ List.map (fun f -> func_type_for m f.it.ftype) m.it.funcs
    in ExternFuncType (nth fts x.it)
  | TableExport x ->
    let tts = tables its @ List.map (fun t -> t.it.ttype) m.it.tables in
    ExternTableType (nth tts x.it)
  | MemoryExport x ->
    let mts = memories its @ List.map (fun m -> m.it.mtype) m.it.memories in
    ExternMemoryType (nth mts x.it)
  | GlobalExport x ->
    let gts = globals its @ List.map (fun g -> g.it.gtype) m.it.globals in
    ExternGlobalType (nth gts x.it)

let string_of_name n =
  let b = Buffer.create 16 in
  let escape uc =
    if uc < 0x20 || uc >= 0x7f then
      Buffer.add_string b (Printf.sprintf "\\u{%02x}" uc)
    else begin
      let c = Char.chr uc in
      if c = '\"' || c = '\\' then Buffer.add_char b '\\';
      Buffer.add_char b c
    end
  in
  List.iter escape n;
  Buffer.contents b

(* is_dwarf_like indicates whether an AST meta instruction
   prevents dead-code elimination. Elimination is forbidden,
   if the instruction contributes to a DIE, i.e. establishes, augments
   or closes a DWARF Tag.
 *)
let rec is_dwarf_like' =
  let open Dwarf5.Meta in
  function
  | Tag _ | TagClose | IntAttribute _ | StringAttribute _ | OffsetAttribute _ -> true
  | Grouped parts -> List.exists is_dwarf_like' parts
  | StatementDelimiter _ | FutureAttribute _ -> false
let is_dwarf_like = function
  | Meta m -> is_dwarf_like' m
  | _ -> false


(* AST traversals *)

let phrase f x = { x with it = f x.it }

let rename_funcs rn : module_' -> module_' = fun m ->
  let var' = rn in
  let var = phrase var' in

  let rec instr' = function
    | Call v -> Call (var v)
    | Block (ty, is) -> Block (ty, instrs is)
    | Loop (ty, is) -> Loop (ty, instrs is)
    | If (ty, is1, is2) -> If (ty, instrs is1, instrs is2)
    | i -> i
  and instr i = phrase instr' i
  and instrs is = List.map instr is in

  let func' f = { f with body = instrs f.body } in
  let func = phrase func' in
  let funcs = List.map func in

  let edesc' = function
    | FuncExport v -> FuncExport (var v)
    | e -> e in
  let edesc = phrase edesc' in
  let export' e = { e with edesc = edesc e.edesc } in
  let export = phrase export' in
  let exports = List.map export in

  let segment' f s = { s with init  = f s.init } in
  let segment f = phrase (segment' f) in

  { m with
    funcs = funcs m.funcs;
    exports = exports m.exports;
    start = Option.map var m.start;
    elems = List.map (segment (List.map var)) m.elems;
  }
