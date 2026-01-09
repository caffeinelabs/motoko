open Parser.MenhirInterpreter

(* In order to print syntax error messages and/or debugging information, we
   need a symbol printer. *)

let abstract abs con = (abs, con)

(* private helper *)
let repr_of_symbol : xsymbol -> (string * string) =
  let simple_token con = (con, con) in
  let binop con = abstract "<binop>" "+" in
  let relop con = abstract "<relop>" "==" in
  let binassign con = abstract "<binassign>" "+=" in
  (* all unary operators are also binary operators, so keep them unary *)
  let unop con = abstract "<unop>" "-" in
  let unassign con = abstract "<unassign>" "-=" in
  (* non-terminal examples: *)
  let eg_exp = "42" in
  let eg_pat = "x" in
  let eg_upat = "X" in
  let eg_typ = "Int" in
  let eg_annot = ": " ^ eg_typ in
  let eg_dec = "let " ^ eg_pat ^ " " ^ eg_annot ^ " = 0" in
  let eg_dec_field = "public " ^ eg_dec in
  let eg_imp = "import Array \"mo:base/Array\"" in
  let eg_case = "case " ^ eg_pat ^ " {}" in
  let eg_exp_field = eg_pat ^ " " ^ eg_annot in
  let eg_exp_obj = String.concat " " ["{"; eg_pat; eg_annot; "}"] in
  let eg_typ_dec = String.concat " " ["type"; eg_upat; "="; eg_typ] in
  let eg_typ_args = "<A, B>" in
  let eg_stab_field = String.concat " " ["stable"; eg_pat; eg_annot] in
  let eg_pre_stab_field = String.concat " " ["in"; eg_pat; eg_annot] in
  let eg_typ_tag = "#t" in
  let seplist a sep = ("seplist(" ^ fst a ^ "," ^ fst sep ^ ")",
                       snd a) in
  let semi = "<semicolon>", ";" in
  let semi2 = ";", ";" in
  let comma = ",", "," in
  let entry_point s = (s, "") in
  function
  | X (T T_error) -> simple_token "error"
  | X (T T_XOROP) -> unop "^"
  | X (T T_XORASSIGN) -> unassign "^="
  | X (T T_WHILE) -> simple_token "while"
  | X (T T_VAR) -> simple_token "var"
  | X (T T_SHROP) -> binop " >>"
  | X (T T_SHRASSIGN) -> binop ">>="
  | X (T T_UNDERSCORE) -> simple_token "_"
  | X (T T_TYPE) -> simple_token  "type"
  | X (T T_TRANSIENT) -> simple_token "transient"
  | X (T T_TRY) -> simple_token "try"
  | X (T T_THROW) -> simple_token "throw"
  | X (T T_FINALLY) -> simple_token  "finally"
  | X (T T_TEXT) -> ("<text>", "\"text\"")
  | X (T T_SWITCH) -> simple_token "switch"
  | X (T T_SUBOP) -> unop "-"
  | X (T T_SUB) -> simple_token "<:"
  | X (T T_STABLE) -> simple_token "stable"
  | X (T T_SHLOP) -> binop "<<"
  | X (T T_SHLASSIGN) -> binassign "<<="
  | X (T T_SHARED) -> simple_token "shared"
  | X (T T_SEMICOLON_EOL) -> simple_token ";" (* suppress the \n *)
  | X (T T_SEMICOLON) -> simple_token ";"
  | X (T T_SYSTEM) -> simple_token "system"
  | X (T T_WITH) -> simple_token "with"
  | X (T T_RPAR) -> simple_token ")"
  | X (T T_ROTROP) -> binop "<>>"
  | X (T T_ROTRASSIGN) -> binassign "<>>="
  | X (T T_ROTLOP) -> binop "<<>"
  | X (T T_ROTLASSIGN) -> binassign "<<>="
  | X (T T_RETURN) -> simple_token "return"
  | X (T T_RCURLY) -> simple_token "}"
  | X (T T_RBRACKET) -> simple_token "]"
  | X (T T_QUEST) -> simple_token "?"
  | X (T T_BANG) -> simple_token "!"
  | X (T T_QUERY) -> simple_token "query"
  | X (T T_PERSISTENT) -> simple_token "persistent"
  | X (T T_PUBLIC) -> simple_token "public"
  | X (T T_PRIVATE) -> simple_token "private"
  | X (T T_PRIM) -> simple_token "prim"
  | X (T T_POWOP) -> binop "**"
  | X (T T_WRAPADDOP) -> binop "+%"
  | X (T T_WRAPSUBOP) -> binop "-%"
  | X (T T_WRAPMULOP) -> binop "*%"
  | X (T T_WRAPPOWOP) -> binop "**%"
  | X (T T_WRAPADDASSIGN) -> binassign "+%-"
  | X (T T_WRAPSUBASSIGN) -> binassign "-%="
  | X (T T_WRAPMULASSIGN) -> binassign "*%="
  | X (T T_WRAPPOWASSIGN) -> binassign "**%="
  | X (T T_POWASSIGN) -> binassign "**="
  | X (T T_PLUSASSIGN) -> unassign "+="
  | X (T T_OROP) -> binop "|"
  | X (T T_ORASSIGN) -> binassign "|="
  | X (T T_OR) -> simple_token "or"
  | X (T T_OBJECT) -> simple_token "object"
  | X (T T_NULL) -> simple_token "null"
  | X (T T_NUM_DOT_ID) -> simple_token "num.id"
  | X (T T_NOT) -> simple_token "not"
  | X (T T_NEQOP) -> binop "!="
  | X (T T_NAT) -> "<nat>", "0"
  | X (T T_MULOP) -> binop "*"
  | X (T T_MULASSIGN) -> binassign "*="
  | X (T T_MODULE) -> simple_token "module"
  | X (T T_INCLUDE) -> simple_token "include"
  | X (T T_MIXIN) -> simple_token "mixin"
  | X (T T_MODOP) -> binop "%"
  | X (T T_MODASSIGN) -> binassign "%="
  | X (T T_MINUSASSIGN) -> unassign "-="
  | X (T T_LTOP) -> relop " < "
  | X (T T_LT) -> simple_token "<"
  | X (T T_LPAR) -> simple_token "("
  | X (T T_LOOP) -> simple_token "loop"
  | X (T T_LET) -> simple_token "let"
  | X (T T_LEOP) -> relop "<="
  | X (T T_LCURLY) -> simple_token "{"
  | X (T T_LBRACKET) -> simple_token "["
  | X (T T_LABEL) -> simple_token "label"
  | X (T T_IN) -> simple_token "in"
  | X (T T_IMPORT) -> simple_token "import"
  | X (T T_IMPLICIT) -> simple_token "implicit"
  | X (T T_IGNORE) -> simple_token "ignore"
  | X (T T_IF) -> simple_token "if"
  | X (T T_ID) -> simple_token "<id>"
  | X (T T_HASH) -> binop "#"
  | X (T T_GTOP) -> relop " > "
  | X (T T_GT) -> simple_token ">"
  | X (T T_GEOP) -> relop ">="
  | X (T T_FUNC) -> simple_token "func"
  | X (T T_FOR) -> simple_token "for"
  | X (T T_FLEXIBLE) -> simple_token "flexible"
  | X (T T_FLOAT) -> simple_token "<float>"
  | X (T T_EQOP) -> relop "=="
  | X (T T_EQ) -> simple_token "="
  | X (T T_EOF) -> simple_token "<eof>"
  | X (T T_ELSE) -> simple_token "else"
  | X (T T_DOT_NUM) -> ".<nat>", ".1"
  | X (T T_DOT) -> simple_token "."
  | X (T T_DO) -> simple_token "do"
  | X (T T_DIVOP) -> binop "/"
  | X (T T_DIVASSIGN) -> binassign "/="
  | X (T T_DISALLOWED) -> simple_token "<disallowed>"
  | X (T T_DEBUG_SHOW) -> simple_token "debug_show"
  | X (T T_TO_CANDID) -> simple_token "to_candid"
  | X (T T_FROM_CANDID) -> simple_token "from_candid"
  | X (T T_DEBUG) -> simple_token "debug"
  | X (T T_CONTINUE) -> simple_token "continue"
  | X (T T_COMMA) -> simple_token ","
  | X (T T_COMPOSITE) -> simple_token "composite"
  | X (T T_COLON) -> simple_token ":"
  | X (T T_CLASS) -> simple_token "class"
  | X (T T_CHAR) ->  "<char>", "'c'"
  | X (T T_CATCH) -> simple_token "catch"
  | X (T T_CATASSIGN) -> binassign "@="
  | X (T T_CASE) -> simple_token "case"
  | X (T T_BREAK) -> simple_token "break"
  | X (T T_BOOL) -> simple_token "<bool>"
  | X (T T_AWAIT) -> simple_token "await"
  | X (T T_AWAITSTAR) -> simple_token "await*"
  | X (T T_AWAITQUEST) -> simple_token "await?"
  | X (T T_ASYNC) -> simple_token "async"
  | X (T T_ASYNCSTAR) -> simple_token "async*"
  | X (T T_ASSIGN) -> simple_token ":="
  | X (T T_ASSERT) -> simple_token "assert"
  | X (T T_ARROW) -> simple_token "->"
  | X (T T_ANDOP) -> binop "&"
  | X (T T_ANDASSIGN) -> binassign "&="
  | X (T T_AND) -> simple_token "and"
  | X (T T_ADDOP) -> unop "+"
  | X (T T_ACTOR) -> simple_token "actor"
  | X (T T_PIPE) -> simple_token "|>"
  | X (T T_WEAK) -> simple_token "weak"
  (* non-terminals *)
  | X (N N_bl) -> "<bl>", "<bl>"
  | X (N N_case) -> "<case>", eg_case
  | X (N N_catch) -> "<catch>", "catch " ^ eg_pat ^ " {}"
  | X (N N_class_body) -> "<class_body>", "= {}"
  | X (N N_dec) -> "<dec>", eg_dec
  | X (N N_dec_field) -> "<dec_field>", eg_dec_field
  | X (N N_dec_nonvar) -> "<dec_nonvar>", eg_dec
  | X (N N_dec_var) -> "<dec_var>", "var x : Int = 0"
  | X (N N_exp_bl_) -> "<exp(bl)>", eg_exp
  | X (N N_exp_ob_) -> "<exp(ob)>", eg_exp
  | X (N N_exp_bin_bl_) -> "<exp_bin(bl)>", eg_exp
  | X (N N_exp_bin_ob_) -> "<exp_bin(ob)>", eg_exp
  | X (N N_exp_obj) -> "<exp_obj>", eg_exp_obj
  | X (N N_exp_nest) -> "<exp_nest>", eg_exp
  | X (N N_block) -> "<block>", "{}"
  | X (N N_exp_field) -> "<exp_field>", eg_exp_field
  | X (N N_exp_nondec_bl_) -> "<exp_nondec(bl)>", eg_exp
  | X (N N_exp_nondec_ob_) -> "<exp_nondec(ob)>", eg_exp
  | X (N N_exp_nonvar_bl_) -> "<exp_nonvar(bl)>", eg_exp
  | X (N N_exp_nonvar_ob_) -> "<exp_nonvar(ob)>", eg_exp
  | X (N N_exp_nullary_bl_) -> "<exp_nullary(bl)>", eg_exp
  | X (N N_exp_nullary_ob_) -> "<exp_nullary(ob)>", eg_exp
  | X (N N_exp_arg) -> "<exp_nullary(ob)>", eg_exp
  | X (N N_exp_plain) -> "<exp_plain>", "true"
  | X (N N_exp_post_bl_) -> "<exp_post(bl)>", eg_exp
  | X (N N_exp_post_ob_) -> "<exp_post(ob)>", eg_exp
  | X (N N_obj_or_class_dec) -> "<obj_or_class_dec>", "object " ^ eg_pat ^ " = {}"
  | X (N N_option_exp_post_ob__) -> "<exp_post(ob)>?", eg_exp
  | X (N N_exp_un_bl_) -> "<exp_un(bl)>", eg_exp
  | X (N N_exp_un_ob_) -> "<exp_un(ob)>", eg_exp
  | X (N N_func_body) -> "<func_body>", "{}"
  | X (N N_func_pat) -> "<func_pat>", "f(x : Int)"
  | X (N N_imp) -> "<imp>", eg_imp
  | X (N N_import_list) -> "<import_list>", eg_imp
  | X (N N_inst) -> "<inst>", "<" ^ eg_typ ^ ">"
  | X (N N_lit) -> "<lit>", "true"
  | X (N N_ob) -> "<ob>", eg_exp_obj
  | X (N N_obj_body) -> "<obj_body>", "{}"
  | X (N N_option_EQ_) -> "=?", "=?"
  | X (N N_option_exp_nullary_ob__) -> "<exp_nullary(ob)>?", eg_exp
  | X (N N_option_typ_args_) -> "<typ_args>?", eg_typ_args
  | X (N N_option_query_) -> "<query>?", "query"
  | X (N N_option_id_) -> "<id>?", "x"
  | X (N N_parse_module_header) -> entry_point "<parse_module_header>"
  | X (N N_parse_prog) -> entry_point "<parse_prog>"
  | X (N N_parse_prog_interactive) -> entry_point "<parse_prog_interactive>"
  | X (N N_parse_stab_sig) -> entry_point "<parse_stab_sig>"
  | X (N N_pat) -> "<pat>", eg_pat
  | X (N N_pat_bin) -> "<pat_bin>", eg_pat
  | X (N N_pat_field) -> "<pat_field>", eg_pat ^ " " ^ eg_pat
  | X (N N_pat_nullary) -> "<pat_nullary>", eg_pat
  | X (N N_pat_plain) -> "<pat_plain>", eg_pat
  | X (N N_pat_un) -> "<pat_un>", eg_pat
  | X (N N_path) -> "<path>", "A.B.C"
  | X (N N_annot_opt) -> "<annot_opt>", eg_annot
  | X (N N_seplist_case_semicolon_) -> seplist ("<case>", eg_case) semi
  | X (N N_seplist_dec_SEMICOLON_) -> seplist ("<dec>", eg_dec) semi2
  | X (N N_seplist_dec_semicolon_) -> seplist ("<dec>", eg_dec) semi
  | X (N N_seplist_typ_dec_semicolon_) -> seplist ("<typ_dec>", eg_typ_dec) semi
  | X (N N_seplist_dec_field_semicolon_) -> seplist ("<dec_field>", eg_dec_field) semi
  | X (N N_seplist_exp_ob__COMMA_) -> seplist ("<exp(ob)>", eg_exp) comma
  | X (N N_seplist_exp_field_semicolon_) -> seplist ("<exp_field>", eg_exp_field) semi
  | X (N N_seplist1_exp_field_semicolon_) -> "seplist1(<exp_field>,<semicolon>)", eg_exp_field
  | X (N N_separated_nonempty_list_AND_exp_post_ob__) -> "seplist+(<exp_post(ob)>,and)", eg_exp
  | X (N N_seplist_exp_nonvar_ob__COMMA_) -> seplist ("<exp_nonvar(ob)>", eg_exp) comma
  | X (N N_seplist_imp_SEMICOLON_) -> seplist ("<imp>", eg_imp) semi2
  | X (N N_seplist_imp_semicolon_) -> seplist ("<imp>", eg_imp) semi
  | X (N N_seplist_pat_bin_COMMA_) -> seplist ("<pat_bin>", eg_pat) comma
  | X (N N_seplist_pat_field_semicolon_) -> seplist ("<pat_field", eg_pat) semi
  | X (N N_seplist_typ_COMMA_) -> seplist ("<typ>", eg_typ) comma
  | X (N N_list_preceded_COMMA_typ__) -> "(, <typ>)*", ", " ^ eg_typ
  | X (N N_seplist_typ_bind_COMMA_) -> seplist ("<typ_bind>", eg_upat) comma
  | X (N N_list_preceded_COMMA_typ_bind__) -> "(, <typ_bind>)*", ", " ^ eg_upat
  | X (N N_seplist_typ_field_semicolon_) -> seplist ("typ_field", eg_typ_dec) semi
  | X (N N_seplist_stab_field_semicolon_) -> seplist ("typ_field", eg_stab_field) semi
  | X (N N_seplist_pre_stab_field_semicolon_) -> seplist ("typ_field", eg_pre_stab_field) semi
  | X (N N_seplist_typ_item_COMMA_) -> seplist ("<typ_item>", eg_typ) comma
  | X (N N_seplist_typ_tag_semicolon_) -> seplist ("<typ_tag>", eg_typ_tag) semi
  | X (N N_seplist1_typ_tag_semicolon_) -> "seplist1(<typ_tag>,<semicolon>)", eg_typ_tag
  | X (N N_pat_opt) -> "<pat_opt>", eg_pat
  | X (N N_typ) -> "<typ>", eg_typ
  | X (N N_typ_dec) -> "<typ_dec>", eg_typ_dec
  | X (N N_typ_args) -> "<typ_args>", eg_typ_args
  | X (N N_typ_bind) -> "<typ_bind>", eg_upat
  | X (N N_typ_field) -> "<typ_field>", eg_typ_dec
  | X (N N_typ_item) -> "<typ_item>", eg_typ
  | X (N N_typ_nobin) -> "<typ_nobin>", eg_typ
  | X (N N_typ_nullary) -> "<typ_nullary>", eg_typ
  | X (N N_typ_obj) -> "<typ_obj>", "{}"
  | X (N N_typ_pre) -> "<typ_pre>", eg_typ
  | X (N N_typ_tag) -> "<typ_tag>", eg_typ_tag
  | X (N N_typ_un) -> "<typ_un>", eg_typ
  | X (N N_typ_variant) -> "<typ_variant>", "{ " ^ eg_typ_tag ^ " }"
  | X (N N_vis) -> "<vis>", "public"
  | X (N N_stab) -> "<stab>", ""
  | X (N N_stab_field) -> "<stab_field>", eg_stab_field
  | X (N N_pre_stab_field) -> "<pre_stab_field>", eg_pre_stab_field
  | X (N N_start) -> entry_point "<start>" (* dummy non-terminal, don't display *)

(* In order to print a view of the stack that includes semantic values,
   we need an element printer. (If we don't need this feature, then
   [print_symbol] above suffices.) *)

(* The public functions. *)


let string_of_symbol s = fst (repr_of_symbol s)
let example_of_symbol s = snd (repr_of_symbol s)

let buff :string list ref = ref []
let print s = buff := s::!buff
let print_symbol s = print (string_of_symbol s)
let print_element = None
let to_string() = let s = String.concat "" (List.rev (!buff)) in
                  buff := [];
                  s
