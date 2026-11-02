(* DWARF 5 constants *)

let dw_FORM_addr = 0x01
(* Reserved = 0x02 *)
let dw_FORM_block2 = 0x03
let dw_FORM_block4 = 0x04
let dw_FORM_data2 = 0x05
let dw_FORM_data4 = 0x06
let dw_FORM_data8 = 0x07
let dw_FORM_string = 0x08
let dw_FORM_block = 0x09
let dw_FORM_block1 = 0x0a
let dw_FORM_data1 = 0x0b
let dw_FORM_flag = 0x0c
let dw_FORM_sdata = 0x0d
let dw_FORM_strp = 0x0e
let dw_FORM_udata = 0x0f
let dw_FORM_ref_addr = 0x10
let dw_FORM_ref1 = 0x11
let dw_FORM_ref2 = 0x12
let dw_FORM_ref4 = 0x13
let dw_FORM_ref8 = 0x14
let dw_FORM_ref_udata = 0x15
let dw_FORM_indirect = 0x16
let dw_FORM_sec_offset = 0x17
let dw_FORM_exprloc = 0x18
let dw_FORM_flag_present = 0x19
let dw_FORM_strx = 0x1a
let dw_FORM_addrx = 0x1b
let dw_FORM_ref_sup4 = 0x1c
let dw_FORM_strp_sup = 0x1d
let dw_FORM_data16 = 0x1e
let dw_FORM_line_strp = 0x1f
let dw_FORM_ref_sig8 = 0x20
let dw_FORM_implicit_const = 0x21
let dw_FORM_loclistx = 0x22
let dw_FORM_rnglistx = 0x23
let dw_FORM_ref_sup8 = 0x24
let dw_FORM_strx1 = 0x25
let dw_FORM_strx2 = 0x26
let dw_FORM_strx3 = 0x27
let dw_FORM_strx4 = 0x28
let dw_FORM_addrx1 = 0x29
let dw_FORM_addrx2 = 0x2a
let dw_FORM_addrx3 = 0x2b
let dw_FORM_addrx4 = 0x2c

(* Line number header entry format name *)
let dw_LNCT_path = 0x1
let dw_LNCT_directory_index = 0x2
let dw_LNCT_timestamp = 0x3
let dw_LNCT_size = 0x4
let dw_LNCT_MD5 = 0x5
let dw_LNCT_lo_user = 0x2000
let dw_LNCT_hi_user = 0x3fff

(* Line number standard opcode encodings *)
let dw_LNS_copy = 0x01
let dw_LNS_advance_pc = 0x02
let dw_LNS_advance_line = 0x03
let dw_LNS_set_file = 0x04
let dw_LNS_set_column = 0x05
let dw_LNS_negate_stmt = 0x06
let dw_LNS_set_basic_block = 0x07
let dw_LNS_const_add_pc = 0x08
let dw_LNS_fixed_advance_pc = 0x09
let dw_LNS_set_prologue_end = 0x0a
let dw_LNS_set_epilogue_begin = 0x0b
let dw_LNS_set_isa = 0x0c

(* Line number extended opcode encodings
   Note: these are negative, so they don't overlap
         with the `dw_LNS_*` above
 *)
let dw_LNE_end_sequence = -0x01
let dw_LNE_set_address = -0x02
(* let Reserved 0x03 *)
let dw_LNE_set_discriminator = -0x04
let dw_LNE_lo_user = 0x80
let dw_LNE_hi_user = 0xff

module Machine =
struct

(* Assumptions:
- op_index = 0 (const)
- maximum_operations_per_instruction = 1 (non-VLIW)
- minimum_instruction_length = 1 (bytecode)
 *)

let default_is_stmt = true
let line_base = 0
let line_range = 7
let opcode_base = dw_LNS_set_isa

type instr_mode = Regular | Prologue | Epilogue
type loc = { file : int; line : int; col : int }

type state = { ip : int
             ; loc : loc
             ; disc : int
             ; stmt : bool
             ; bb : bool
             ; mode : instr_mode }

(*
Legend:
-------
ip: instruction pointer (Wasm bytecode offset in CODE section)
loc: source location, file encoded as an index
disc(riminator): instance of inlined code fragment (not relevant yet)
--flags actionable (by debugger) bits of information (regarding stopping)
stmt: statement
bb: basic block
mode: how the instruction should be treated

See "6.2 Line Number Information" for details.
*)
let default_loc = { file = 1; line = 1; col = 0 }
let default_flags = default_is_stmt, false, Prologue
(* Table 6.4: Line number program initial state *)
let start_state = { ip = 0; loc = default_loc; disc = 0; stmt = default_is_stmt; bb = false; mode = Prologue }


(* Infers a list of opcodes for the line number program
   ("6.2.5 The Line Number Program") that, when run, would
   transition the machine from a certain intermediate state
   to a following state. This is intended to be used in a loop
   (fold) to obtain all the opcodes for a list of states.
*)
let rec infer from toward = match from, toward with
  | f, {ip; _} when ip < f.ip -> failwith "can't go backwards"
  | {ip=0; _}, t when t.ip > 0 ->
    dw_LNE_set_address :: t.ip :: infer {from with ip = t.ip} t
  | {ip; _}, t when t.ip > ip ->
    dw_LNS_advance_pc :: t.ip - ip :: infer {from with ip = t.ip} t
  | {loc; _}, {loc = {file; _}; _} when file <> loc.file ->
    dw_LNS_set_file :: file :: infer {from with loc = {loc with file}} toward
  | {loc; _}, {loc = {line; _}; _} when line <> loc.line ->
    dw_LNS_advance_line :: line - loc.line :: infer {from with loc = {loc with line}} toward
  | {loc; _}, {loc = {col; _}; _} when col <> loc.col ->
    dw_LNS_set_column :: col :: infer {from with loc = {loc with col}} toward
  | {disc; _}, _ when disc <> toward.disc -> failwith "cannot do disc yet"
  | {stmt; _}, _ when stmt <> toward.stmt ->
    dw_LNS_negate_stmt :: infer {from with stmt = toward.stmt} toward
  | {bb; _}, _ when bb <> toward.bb -> failwith "cannot do bb yet"
  | {mode = Prologue; _}, {mode = Regular; _} ->
    dw_LNS_set_prologue_end :: infer toward toward
  | {mode = Regular; _}, {mode = Epilogue; _} ->
    dw_LNS_set_epilogue_begin :: infer toward toward
  | {mode = Prologue; _}, {mode = Epilogue; _} ->
    dw_LNS_set_prologue_end :: dw_LNS_set_epilogue_begin :: infer toward toward
  | state, state' when state = state' -> [dw_LNS_copy]
  | _ -> failwith "not covered"

(* Given a few formatted outputter functions, dump the contents
   of a line program (essentially a list of `DW_LNS/E_*` opcodes with
   arguments). The bottleneck functions are expected to close over
   the output buffer/stream.
*)
let write_opcodes u8 uleb sleb u32 : int list -> unit =
  let standard lns = u8 lns in
  let extended1 lne = u8 0; u8 1; u8 (- lne) in
  let extended5 lne = u8 0; u8 5; u8 (- lne) in
  let rec chase = function
  | [] -> ()
  | op :: tail when dw_LNS_copy = op -> standard op; chase tail
  | op :: offs :: tail when dw_LNS_advance_pc = op -> standard op; uleb offs; chase tail
  | op :: delta :: tail when dw_LNS_advance_line = op -> standard op; sleb delta; chase tail
  | op :: file :: tail when dw_LNS_set_file = op -> standard op; uleb file; chase tail
  | op :: col :: tail when dw_LNS_set_column = op -> standard op; uleb col; chase tail
  | op :: tail when dw_LNS_negate_stmt = op -> standard op; chase tail
  | op :: tail when dw_LNS_set_prologue_end = op -> standard op; chase tail
  | op :: tail when dw_LNS_set_epilogue_begin = op -> standard op; chase tail
  | op :: tail when dw_LNE_end_sequence = op -> extended1 op; chase tail
  | op :: addr :: tail when dw_LNE_set_address = op -> extended5 op; u32 addr; chase tail
  | op :: _ -> failwith (Printf.sprintf "opcode not covered: %d" op)
  in chase

end
