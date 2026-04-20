(* DWARF 5 constants *)

let dw_CHILDREN_no = 0x00
let dw_CHILDREN_yes = 0x01

let dw_TAG_array_type = 0x01
let dw_TAG_class_type = 0x02
let dw_TAG_entry_point = 0x03
let dw_TAG_enumeration_type = 0x04
let dw_TAG_formal_parameter = 0x05
(* let Reserved = 0x06 *)
(* let Reserved = 0x07 *)
let dw_TAG_imported_declaration = 0x08
(* let Reserved = 0x09 *)
let dw_TAG_label = 0x0a
let dw_TAG_lexical_block = 0x0b
(* let Reserved = 0x0c *)
let dw_TAG_member = 0x0d
(* let Reserved = 0x0e *)
let dw_TAG_pointer_type = 0x0f
let dw_TAG_reference_type = 0x10
let dw_TAG_compile_unit = 0x11
let dw_TAG_string_type = 0x12
let dw_TAG_structure_type = 0x13
(* let Reserved = 0x14 *)
let dw_TAG_subroutine_type = 0x15
let dw_TAG_typedef = 0x16
let dw_TAG_union_type = 0x17
let dw_TAG_unspecified_parameters = 0x18
let dw_TAG_variant = 0x19
let dw_TAG_common_block = 0x1a
let dw_TAG_common_inclusion = 0x1b
let dw_TAG_inheritance = 0x1c
let dw_TAG_inlined_subroutine = 0x1d
let dw_TAG_module = 0x1e
let dw_TAG_ptr_to_member_type = 0x1f
let dw_TAG_set_type = 0x20
let dw_TAG_subrange_type = 0x21
let dw_TAG_with_stmt = 0x22
let dw_TAG_access_declaration = 0x23
let dw_TAG_base_type = 0x24
let dw_TAG_catch_block = 0x25
let dw_TAG_const_type = 0x26
let dw_TAG_constant = 0x27
let dw_TAG_enumerator = 0x28
let dw_TAG_file_type = 0x29
let dw_TAG_friend = 0x2a
let dw_TAG_namelist = 0x2b
let dw_TAG_namelist_item = 0x2c
let dw_TAG_packed_type = 0x2d
let dw_TAG_subprogram = 0x2e
let dw_TAG_template_type_parameter = 0x2f
let dw_TAG_template_value_parameter = 0x30
let dw_TAG_thrown_type = 0x31
let dw_TAG_try_block = 0x32
let dw_TAG_variant_part = 0x33
let dw_TAG_variable = 0x34
let dw_TAG_volatile_type = 0x35
let dw_TAG_dwarf_procedure = 0x36
let dw_TAG_restrict_type = 0x37
let dw_TAG_interface_type = 0x38
let dw_TAG_namespace = 0x39
let dw_TAG_imported_module = 0x3a
let dw_TAG_unspecified_type = 0x3b
let dw_TAG_partial_unit = 0x3c
let dw_TAG_imported_unit = 0x3d
(* let Reserved = 0x3e *)
let dw_TAG_condition = 0x3f
let dw_TAG_shared_type = 0x40
let dw_TAG_type_unit = 0x41
let dw_TAG_rvalue_reference_type = 0x42
let dw_TAG_template_alias = 0x43
let dw_TAG_coarray_type = 0x44
let dw_TAG_generic_subrange = 0x45
let dw_TAG_dynamic_type = 0x46
let dw_TAG_atomic_type = 0x47
let dw_TAG_call_site = 0x48
let dw_TAG_call_site_parameter = 0x49
let dw_TAG_skeleton_unit = 0x4a
let dw_TAG_immutable_type = 0x4b
let dw_TAG_lo_user = 0x4080
let dw_TAG_hi_user = 0xffff


let dw_AT_sibling = 0x01
let dw_AT_location = 0x02
let dw_AT_name = 0x03
(* let Reserved = 0x04 *)
(* let Reserved = 0x05 *)
(* let Reserved = 0x06 *)
(* let Reserved = 0x07 *)
(* let Reserved = 0x08 *)
let dw_AT_ordering = 0x09
(* let Reserved = 0x0a *)
let dw_AT_byte_size = 0x0b
(* let Reserved = 0x0c *)
let dw_AT_bit_size = 0x0d
(* let Reserved = 0x0e *)
(* let Reserved = 0x0f *)
let dw_AT_stmt_list = 0x10
let dw_AT_low_pc = 0x11
let dw_AT_high_pc = 0x12
let dw_AT_language = 0x13
(* let Reserved = 0x14 *)
let dw_AT_discr = 0x15
let dw_AT_discr_value = 0x16
let dw_AT_visibility = 0x17
let dw_AT_import = 0x18
let dw_AT_string_length = 0x19
let dw_AT_common_reference = 0x1a
let dw_AT_comp_dir = 0x1b
let dw_AT_const_value = 0x1c
let dw_AT_containing_type = 0x1d
let dw_AT_default_value = 0x1e
(* let Reserved = 0x1f *)
let dw_AT_inline = 0x20
let dw_AT_is_optional = 0x21
let dw_AT_lower_bound = 0x22
(* let Reserved = 0x23 *)
(* let Reserved = 0x24 *)
let dw_AT_producer = 0x25
(* let Reserved = 0x26 *)
let dw_AT_prototyped = 0x27
(* let Reserved = 0x28 *)
(* let Reserved = 0x29 *)
let dw_AT_return_addr = 0x2a
(* let Reserved = 0x2b *)
let dw_AT_start_scope = 0x2c
(* let Reserved = 0x2d *)
let dw_AT_bit_stride = 0x2e
let dw_AT_upper_bound = 0x2f
(* let Reserved = 0x30 *)
let dw_AT_abstract_origin = 0x31
let dw_AT_accessibility = 0x32
let dw_AT_address_class = 0x33
let dw_AT_artificial = 0x34
let dw_AT_base_types = 0x35
let dw_AT_calling_convention = 0x36
let dw_AT_count = 0x37
let dw_AT_data_member_location = 0x38
let dw_AT_decl_column = 0x39
let dw_AT_decl_file = 0x3a
let dw_AT_decl_line = 0x3b
let dw_AT_declaration = 0x3c
let dw_AT_discr_list = 0x3d
let dw_AT_encoding = 0x3e
let dw_AT_external = 0x3f
let dw_AT_frame_base = 0x40
let dw_AT_friend = 0x41
let dw_AT_identifier_case = 0x42
(* let Reserved = 0x43 *)
let dw_AT_namelist_item = 0x44
let dw_AT_priority = 0x45
let dw_AT_segment = 0x46
let dw_AT_specification = 0x47
let dw_AT_static_link = 0x48
let dw_AT_type = 0x49
let dw_AT_use_location = 0x4a
let dw_AT_variable_parameter = 0x4b
let dw_AT_virtuality = 0x4c
let dw_AT_vtable_elem_location = 0x4d
let dw_AT_allocated = 0x4e
let dw_AT_associated = 0x4f
let dw_AT_data_location = 0x50
let dw_AT_byte_stride = 0x51
let dw_AT_entry_pc = 0x52
let dw_AT_use_UTF8 = 0x53
let dw_AT_extension = 0x54
let dw_AT_ranges = 0x55
let dw_AT_trampoline = 0x56
let dw_AT_call_column = 0x57
let dw_AT_call_file = 0x58
let dw_AT_call_line = 0x59
let dw_AT_description = 0x5a
let dw_AT_binary_scale = 0x5b
let dw_AT_decimal_scale = 0x5c
let dw_AT_small = 0x5d
let dw_AT_decimal_sign = 0x5e
let dw_AT_digit_count = 0x5f
let dw_AT_picture_string = 0x60
let dw_AT_mutable = 0x61
let dw_AT_threads_scaled = 0x62
let dw_AT_explicit = 0x63
let dw_AT_object_pointer = 0x64
let dw_AT_endianity = 0x65
let dw_AT_elemental = 0x66
let dw_AT_pure = 0x67
let dw_AT_recursive = 0x68
let dw_AT_signature = 0x69
let dw_AT_main_subprogram = 0x6a
let dw_AT_data_bit_offset = 0x6b
let dw_AT_const_expr = 0x6c
let dw_AT_enum_class = 0x6d
let dw_AT_linkage_name = 0x6e
let dw_AT_string_length_bit_size = 0x6f
let dw_AT_string_length_byte_size = 0x70
let dw_AT_rank = 0x71
let dw_AT_str_offsets_base = 0x72
let dw_AT_addr_base = 0x73
let dw_AT_rnglists_base = 0x74
(* let Reserved = 0x75 *)
let dw_AT_dwo_name = 0x76
let dw_AT_reference = 0x77
let dw_AT_rvalue_reference = 0x78
let dw_AT_macros = 0x79
let dw_AT_call_all_calls = 0x7a
let dw_AT_call_all_source_calls = 0x7b
let dw_AT_call_all_tail_calls = 0x7c
let dw_AT_call_return_pc = 0x7d
let dw_AT_call_value = 0x7e
let dw_AT_call_origin = 0x7f
let dw_AT_call_parameter = 0x80
let dw_AT_call_pc = 0x81
let dw_AT_call_tail_call = 0x82
let dw_AT_call_target = 0x83
let dw_AT_call_target_clobbered = 0x84
let dw_AT_call_data_location = 0x85
let dw_AT_call_data_value = 0x86
let dw_AT_noreturn = 0x87
let dw_AT_alignment = 0x88
let dw_AT_export_symbols = 0x89
let dw_AT_deleted = 0x8a
let dw_AT_defaulted = 0x8b
let dw_AT_loclists_base = 0x8c
let dw_AT_lo_user = 0x2000
let dw_AT_hi_user = 0x3fff

let dw_ATE_address = 0x01
let dw_ATE_boolean = 0x02
let dw_ATE_complex_float = 0x03
let dw_ATE_float = 0x04
let dw_ATE_signed = 0x05
let dw_ATE_signed_char = 0x06
let dw_ATE_unsigned = 0x07
let dw_ATE_unsigned_char = 0x08
let dw_ATE_imaginary_float = 0x09
let dw_ATE_packed_decimal = 0x0a
let dw_ATE_numeric_string = 0x0b
let dw_ATE_edited = 0x0c
let dw_ATE_signed_fixed = 0x0d
let dw_ATE_unsigned_fixed = 0x0e
let dw_ATE_decimal_float = 0x0f
let dw_ATE_UTF = 0x10
let dw_ATE_UCS = 0x11
let dw_ATE_ASCII = 0x12
let dw_ATE_lo_user = 0x80
let dw_ATE_hi_user = 0xff

let dw_DS_unsigned = 0x01
let dw_DS_leading_overpunch = 0x02
let dw_DS_trailing_overpunch = 0x03
let dw_DS_leading_separate = 0x04
let dw_DS_trailing_separate = 0x05

let dw_END_default = 0x00
let dw_END_big = 0x01
let dw_END_little = 0x02
let dw_END_lo_user = 0x40
let dw_END_hi_user = 0xff
  
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

(* FIXME: we cheat for now *)
let dw_LANG_C99 = 0x000c
let dw_LANG_Swift = 0x001e
let dw_LANG_Motoko = 0x0027


let dw_UT_compile = 0x01
let dw_UT_type = 0x02
let dw_UT_partial = 0x03
let dw_UT_skeleton = 0x04
let dw_UT_split_compile = 0x05
let dw_UT_split_type = 0x06
let dw_UT_lo_user = 0x80
let dw_UT_hi_user = 0xff

let dw_RLE_end_of_list = 0x00
let dw_RLE_base_addressx = 0x01
let dw_RLE_startx_endx = 0x02
let dw_RLE_startx_length = 0x03
let dw_RLE_offset_pair = 0x04
let dw_RLE_base_address = 0x05
let dw_RLE_start_end = 0x06
let dw_RLE_start_length = 0x07

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

(* DWARF expression opcode encodings *)
let dw_OP_addr = 0x03
let dw_OP_deref = 0x06
let dw_OP_const1u = 0x08
let dw_OP_const1s = 0x09
let dw_OP_const2u = 0x0a
let dw_OP_const2s = 0x0b
let dw_OP_const4u = 0x0c
let dw_OP_const4s = 0x0d
let dw_OP_const8u = 0x0e
let dw_OP_const8s = 0x0f
let dw_OP_constu = 0x10
let dw_OP_consts = 0x11
let dw_OP_dup = 0x12
let dw_OP_drop = 0x13
let dw_OP_over = 0x14
let dw_OP_pick = 0x15
let dw_OP_swap = 0x16
let dw_OP_rot = 0x17
let dw_OP_xderef = 0x18
let dw_OP_abs = 0x19
let dw_OP_and = 0x1a
let dw_OP_div = 0x1b
let dw_OP_minus = 0x1c
let dw_OP_mod = 0x1d
let dw_OP_mul = 0x1e
let dw_OP_neg = 0x1f
let dw_OP_not = 0x20
let dw_OP_or = 0x21
let dw_OP_plus = 0x22
let dw_OP_plus_uconst = 0x23
let dw_OP_shl = 0x24
let dw_OP_shr = 0x25
let dw_OP_shra = 0x26
let dw_OP_xor = 0x27
let dw_OP_skip = 0x2f
let dw_OP_bra = 0x28
let dw_OP_eq = 0x29
let dw_OP_ge = 0x2a
let dw_OP_gt = 0x2b
let dw_OP_le = 0x2c
let dw_OP_lt = 0x2d
let dw_OP_ne = 0x2e
let dw_OP_lit0 = 0x30
let dw_OP_lit1 = 0x31
let dw_OP_lit2 = 0x32
let dw_OP_lit3 = 0x33
let dw_OP_lit4 = 0x34
let dw_OP_lit5 = 0x35
let dw_OP_lit6 = 0x36
let dw_OP_lit7 = 0x37
let dw_OP_lit8 = 0x38
let dw_OP_lit9 = 0x39
let dw_OP_lit10 = 0x3a
let dw_OP_lit11 = 0x3b
let dw_OP_lit12 = 0x3c
let dw_OP_lit13 = 0x3d
let dw_OP_lit14 = 0x3e
let dw_OP_lit15 = 0x3f
let dw_OP_lit16 = 0x40
let dw_OP_lit17 = 0x41
let dw_OP_lit18 = 0x42
let dw_OP_lit19 = 0x43
let dw_OP_lit20 = 0x44
let dw_OP_lit21 = 0x45
let dw_OP_lit22 = 0x46
let dw_OP_lit23 = 0x47
let dw_OP_lit24 = 0x48
let dw_OP_lit25 = 0x49
let dw_OP_lit26 = 0x4a
let dw_OP_lit27 = 0x4b
let dw_OP_lit28 = 0x4c
let dw_OP_lit29 = 0x4d
let dw_OP_lit30 = 0x4e
let dw_OP_lit31 = 0x4f
let dw_OP_reg0 = 0x50
let dw_OP_reg1 = 0x51
let dw_OP_reg2 = 0x52
let dw_OP_reg3 = 0x53
let dw_OP_reg4 = 0x54
let dw_OP_reg5 = 0x55
let dw_OP_reg6 = 0x56
let dw_OP_reg7 = 0x57
let dw_OP_reg8 = 0x58
let dw_OP_reg9 = 0x59
let dw_OP_reg10 = 0x5a
let dw_OP_reg11 = 0x5b
let dw_OP_reg12 = 0x5c
let dw_OP_reg13 = 0x5d
let dw_OP_reg14 = 0x5e
let dw_OP_reg15 = 0x5f
let dw_OP_reg16 = 0x60
let dw_OP_reg17 = 0x61
let dw_OP_reg18 = 0x62
let dw_OP_reg19 = 0x63
let dw_OP_reg20 = 0x64
let dw_OP_reg21 = 0x65
let dw_OP_reg22 = 0x66
let dw_OP_reg23 = 0x67
let dw_OP_reg24 = 0x68
let dw_OP_reg25 = 0x69
let dw_OP_reg26 = 0x6a
let dw_OP_reg27 = 0x6b
let dw_OP_reg28 = 0x6c
let dw_OP_reg29 = 0x6d
let dw_OP_reg30 = 0x6e
let dw_OP_reg31 = 0x6f
let dw_OP_breg0 = 0x70
let dw_OP_breg1 = 0x71
let dw_OP_breg2 = 0x72
let dw_OP_breg3 = 0x73
let dw_OP_breg4 = 0x74
let dw_OP_breg5 = 0x75
let dw_OP_breg6 = 0x76
let dw_OP_breg7 = 0x77
let dw_OP_breg8 = 0x78
let dw_OP_breg9 = 0x79
let dw_OP_breg10 = 0x7a
let dw_OP_breg11 = 0x7b
let dw_OP_breg12 = 0x7c
let dw_OP_breg13 = 0x7d
let dw_OP_breg14 = 0x7e
let dw_OP_breg15 = 0x7f
let dw_OP_breg16 = 0x80
let dw_OP_breg17 = 0x81
let dw_OP_breg18 = 0x82
let dw_OP_breg19 = 0x83
let dw_OP_breg20 = 0x84
let dw_OP_breg21 = 0x85
let dw_OP_breg22 = 0x86
let dw_OP_breg23 = 0x87
let dw_OP_breg24 = 0x88
let dw_OP_breg25 = 0x89
let dw_OP_breg26 = 0x8a
let dw_OP_breg27 = 0x8b
let dw_OP_breg28 = 0x8c
let dw_OP_breg29 = 0x8d
let dw_OP_breg30 = 0x8e
let dw_OP_breg31 = 0x8f
let dw_OP_regx = 0x90
let dw_OP_fbreg = 0x91
let dw_OP_bregx = 0x92
let dw_OP_piece = 0x93
let dw_OP_deref_size = 0x94
let dw_OP_xderef_size = 0x95
let dw_OP_nop = 0x96
let dw_OP_push_object_address = 0x97
let dw_OP_call2 = 0x98
let dw_OP_call4 = 0x99
let dw_OP_call_ref = 0x9a
let dw_OP_form_tls_address = 0x9b
let dw_OP_call_frame_cfa = 0x9c
let dw_OP_bit_piece = 0x9d
let dw_OP_implicit_value = 0x9e
let dw_OP_stack_value = 0x9f
let dw_OP_implicit_pointer = 0xa0
let dw_OP_addrx = 0xa1
let dw_OP_constx = 0xa2
let dw_OP_entry_value = 0xa3
let dw_OP_const_type = 0xa4
let dw_OP_regval_type = 0xa5
let dw_OP_deref_type = 0xa6
let dw_OP_xderef_type = 0xa7
let dw_OP_convert = 0xa8
let dw_OP_reinterpret = 0xa9

let dw_OP_lo_user = 0xe0
let dw_OP_hi_user = 0xff

let dw_OP_WASM_location = 0xed (* see module Location, below *)

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

module Location =
struct
(* Wasm extensions for DWARF expressions are currently defined

in https://yurydelendik.github.io/webassembly-dwarf/#DWARF-expressions-and-location-descriptions

DW_OP_WASM_location := 0xED ;; available DWARF extension code

wasm-op := wasm-local | wasm-global | wasm-operand-stack

wasm-local := 0x00 i:uleb128
wasm-global := 0x01 i:uleb128
wasm-operand-stack := 0x02
*)

(* Difference-lists-based builder for location programs
   ("2.5 DWARF Expressions") consisting from `DW_OP_*` opcodes
   and their arguments. Since the final output is bytecode
   (i.e. list of 0..255), we use the convention that negative
   integers (`-i`) in the list externalise as ULEB128-encoded
   bytes  of `i`.
*)

let local slot (rest : int list) : int list =
  dw_OP_WASM_location :: 0x00 :: -slot :: rest

(* below two expressions are not supported yet *)

let global slot rest =
  dw_OP_WASM_location :: 0x01 :: -slot :: rest

let operand_stack slot rest =
  dw_OP_WASM_location :: 0x02 :: -slot :: rest

end


module Meta =
struct

type die
  = StatementDelimiter of Source.pos
  | Tag of int option * int * die list
  | TagClose
  | OffsetAttribute of int
  | IntAttribute of int * int
  | StringAttribute of int * string
  (* below is after peephole optimisation only, snoc-like *)
  | Grouped of die list
  (* below is encoding-time only *)
  | FutureAttribute of (unit -> die)

end
