%{
open Mo_config
open Mo_def
open Mo_types
open Mo_values

open Syntax
open Source
open Operator
open Parser_lib


(* Position handling *)

let position_to_pos position =
  { file = position.Lexing.pos_fname;
    line = position.Lexing.pos_lnum;
    column = position.Lexing.pos_cnum - position.Lexing.pos_bol
  }

let positions_to_region position1 position2 =
  { left = position_to_pos position1;
    right = position_to_pos position2
  }

let at (startpos, endpos) = positions_to_region startpos endpos

let syntax_error at code msg =
  Diag.add_msg (Option.get !msg_store)
    (Diag.error_message at code "syntax" msg)

(* Helpers *)

let persistent bool at = { it = bool; at = at; note = [] }

let scope_bind x at =
  { var = Type.scope_var x @@ at;
    sort = Type.Scope @@ at;
    bound = PrimT "Any" @! at
  } @= at

let ensure_scope_bind var tbs =
  match tbs with
  | tb::_ when tb.it.sort.it = Type.Scope -> tbs
  | _ -> scope_bind var no_region :: tbs

let ensure_async_typ t_opt =
  match t_opt with
  | None -> t_opt
  | Some { it = AsyncT _; _} -> t_opt
  | Some t -> Some (AsyncT(Type.Fut, scopeT no_region, t) @! no_region)

let funcT (sort, tbs, t1, t2) =
  match sort.it, t2.it with
  | Type.Local, AsyncT _ -> FuncT (sort, ensure_scope_bind "" tbs, t1, t2)
  | Type.Shared _, _ -> FuncT (sort, ensure_scope_bind "" tbs, t1, t2)
  | _ -> FuncT(sort, tbs, t1, t2)


let dup_var x = VarE (x.it @~ x.at) @? x.at

let name_exp e =
  match e.it with
  | VarE x -> [], e, dup_var x
  | _ ->
    let x = anon_id "val" e.at @@ e.at in
    [LetD (VarP x @! x.at, e, None) @? e.at], dup_var x, dup_var x

let assign_op lhs rhs_f at =
  let ds, lhs', rhs' =
    match lhs.it with
    | VarE x -> [], lhs, dup_var x
    | DotE (e1, x, n) ->
      let ds, ex11, ex12 = name_exp e1 in
      ds, DotE (ex11, x, n) @? lhs.at, DotE (ex12, x.it @@ x.at, n) @? lhs.at
    | IdxE (e1, e2) ->
      let ds1, ex11, ex12 = name_exp e1 in
      let ds2, ex21, ex22 = name_exp e2 in
      ds1 @ ds2, IdxE (ex11, ex21) @? lhs.at, IdxE (ex12, ex22) @? lhs.at
    | _ ->
      name_exp lhs
  in
  let e = AssignE (lhs', rhs_f rhs') @? at in
  match ds with
  | [] -> e
  | ds -> BlockE (ds @ [ExpD e @? e.at]) @? at

let no_inst () : inst = { it = None; at = no_region; note = [] }

let annot_exp e = function
  | None -> e
  | Some t -> AnnotE(e, t) @? span t.at e.at

let annot_pat p = function
  | None -> p
  | Some t -> AnnotP(p, t) @! span t.at p.at


let rec normalize_let p e =
    match p.it with
    | AnnotP(p', t) -> p', AnnotE(e, t) @? p.at
    | ParP p' -> normalize_let p' e
    | _ -> (p, e)

let let_or_exp named x e' at =
  if named
  then LetD(VarP x @! x.at, e' @? at, None) @? at
       (* If you change the above regions,
          modify is_sugared_func_or_module to match *)
  else ExpD(e' @? at) @? at

let is_sugared_func_or_module dec = match dec.it with
  | LetD({it = VarP _; _}, exp, None) ->
    dec.at = exp.at &&
    (match exp.it with
    | ObjBlockE (_, sort, _, _) ->
      sort.it = Type.Module
    | FuncE _ ->
      true
    | _ -> false
    )
  | _ -> false


let func_exp f s tbs p t_opt is_sugar e =
  match s.it, t_opt, e with
  | Type.Local, Some {it = AsyncT _; _}, {it = AsyncE _; _}
  | Type.Shared _, _, _ ->
    FuncE(f, s, ensure_scope_bind "" tbs, p, t_opt, is_sugar, e)
  | _ ->
    FuncE(f, s, tbs, p, t_opt, is_sugar, e)

let desugar_func_body sp x t_opt (is_sugar, e) =
  if not is_sugar then
    false, e (* body declared as EQ e *)
  else (* body declared as immediate block *)
    match sp.it, t_opt with
    | _, Some {it = AsyncT (s, _, _); _} ->
      true, asyncE s (scope_bind x.it e.at) e
    | Type.Shared _, (None | Some { it = TupT []; _}) ->
      true, ignore_asyncE (scope_bind x.it e.at) e
    | _, _ -> (true, e)

let share_typ t =
  match t.it with
  | FuncT ({it = Type.Local; _} as s, tbs, t1, t2) ->
    { t with it = funcT ({s with it = Type.Shared Type.Write}, tbs, t1, t2)}
  | _ -> t

let share_typfield' = function
  | TypF (c, tps, t) -> TypF (c, tps, t)
  | ValF (x, t, m) -> ValF (x, share_typ t, m)

let share_typfield (tf : typ_field) = { tf with it = share_typfield' tf.it }

let share_exp e =
  match e.it with
  | FuncE (x, ({it = Type.Local; _} as sp), tbs, p,
    ((None | Some { it = TupT []; _ }) as t_opt), true, e) ->
    func_exp x {sp with it = Type.Shared (Type.Write, WildP @! sp.at)} tbs p t_opt true (ignore_asyncE (scope_bind x e.at) e) @? e.at
  | FuncE (x, ({it = Type.Local; _} as sp), tbs, p, t_opt, s, e) ->
    func_exp x {sp with it = Type.Shared (Type.Write, WildP @! sp.at)} tbs p t_opt s e @? e.at
  | _ -> e

let share_dec d =
  match d.it with
  | LetD (p, e, f) -> LetD (p, share_exp e, f) @? d.at
  | _ -> d

let share_stab default_stab stab_opt dec =
  match stab_opt with
  | None ->
    (match dec.it with
     | VarD _
     | LetD _ -> Some (default_stab ())
     | _ -> None)
  | _ -> stab_opt

let ensure_system_cap (df : dec_field) =
  match df.it.dec.it with
    | LetD ({ it = VarP { it = "preupgrade" | "postupgrade"; _}; _} as pat, ({ it = FuncE (x, sp, tbs, p, t_opt, s, e); _ } as value), other) ->
      let it = LetD (pat, { value with it = FuncE (x, sp, ensure_scope_bind "" tbs, p, t_opt, s, e) }, other) in
      { df with it = { df.it with dec = { df.it.dec with it } } }
    | _ -> df

let share_dec_field default_stab (df : dec_field) =
  match df.it.vis.it with
  | Public _ ->
    {df with it = {df.it with
      dec = share_dec df.it.dec;
      stab = share_stab (fun () -> Flexible @@ df.it.dec.at) df.it.stab df.it.dec}}
  | System -> ensure_system_cap df
  | _ when is_sugared_func_or_module (df.it.dec) ->
    {df with it =
       {df.it with stab =
          match df.it.stab with
          | None -> Some (Flexible @@ df.it.dec.at)
          | some -> some}
    }
  | _ ->
    {df with it =
       {df.it with stab =
          match df.it.stab with
          | None ->
             (match df.it.dec.it with
             | ExpD _
             | TypD _
             | MixinD _
             | ClassD _ -> None
             | _ -> Some (default_stab()))
          | some -> some}
    }


and objblock eo s id ty dec_fields =
  List.iter (fun df ->
    match df.it.vis.it, df.it.dec.it with
    | Public _, ClassD (_, _, _, id, _, _, _, _, _) when is_anon_id id ->
      syntax_error df.it.dec.at "M0158" "a public class cannot be anonymous, please provide a name"
    | _ -> ()) dec_fields;
  ObjBlockE(eo, s, (id, ty), dec_fields)

%}

%token EOF DISALLOWED

%token LET VAR
%token LPAR RPAR LBRACKET RBRACKET LCURLY RCURLY
(* `(`/`[` not preceded by whitespace; in scrutinee/condition position only
   these extend the expression (call/index), a spaced one starts the body *)
%token TIGHT_LPAR TIGHT_LBRACKET
(* `#` immediately followed by an identifier: a variant introduction, never
   the (binary) concatenation operator *)
%token TIGHT_HASH
%token AWAIT AWAITSTAR AWAITQUEST ASYNC ASYNCSTAR BREAK CASE CATCH CONTINUE DO LABEL DEBUG
%token IF IGNORE IN IMPLICIT ELSE SWITCH LOOP WHILE FOR RETURN TRY THROW FINALLY WITH
%token ARROW ASSIGN
%token FUNC TYPE OBJECT ACTOR CLASS PUBLIC PRIVATE SHARED SYSTEM QUERY
%token SEMICOLON SEMICOLON_EOL COMMA COLON SUB DOT QUEST BANG
%token AND OR NOT
%token IMPORT INCLUDE MODULE MIXIN
%token DEBUG_SHOW
%token TO_CANDID FROM_CANDID
%token ASSERT
%token ADDOP SUBOP MULOP DIVOP MODOP POWOP
%token WRAPADDOP WRAPSUBOP WRAPMULOP WRAPPOWOP
%token ANDOP OROP XOROP SHLOP SHROP ROTLOP ROTROP
%token EQOP NEQOP LEOP LTOP GTOP GEOP
%token HASH
%token EQ LT GT
%token PLUSASSIGN MINUSASSIGN MULASSIGN DIVASSIGN MODASSIGN POWASSIGN CATASSIGN
%token ANDASSIGN ORASSIGN XORASSIGN SHLASSIGN SHRASSIGN ROTLASSIGN ROTRASSIGN
%token WRAPADDASSIGN WRAPSUBASSIGN WRAPMULASSIGN WRAPPOWASSIGN
%token NULL
%token NULLCOALESCE
%token FLEXIBLE STABLE
%token TRANSIENT PERSISTENT
%token<string> DOT_NUM
%token<string> NAT
%token<string * string> NUM_DOT_ID
%token<string> FLOAT
%token<Mo_values.Value.unicode> CHAR
%token<bool> BOOL
%token<string> ID [@recover.expr "__error_recovery_var__"]
%token<string> TEXT
%token PIPE
%token PRIM
%token UNDERSCORE
%token COMPOSITE
%token WEAK

(* EXP_NO_JUXTA marks reductions that end an expression (or pattern) where the
   next token could instead extend it (call argument, indexing, variant
   payload, operand of an operator). It sits below every token, so the parser
   always prefers to extend ("greedy" parsing, as in Rust): in `if f (x) { }`
   the `(x)` is a call argument, not the branch. The highest precedence level
   (below, after the operators) lists the tokens that can start such an
   extension or begin the body following an unparenthesized scrutinee. *)
%nonassoc EXP_NO_JUXTA
%nonassoc RETURN_NO_ARG IF_NO_ELSE LOOP_NO_WHILE TRY_CATCH_NO_FINALLY
%nonassoc ELSE WHILE FINALLY

%left COLON
%left PIPE
%left OR
%left AND
%nonassoc EQOP NEQOP LEOP LTOP GTOP GEOP
%left ADDOP SUBOP WRAPADDOP WRAPSUBOP HASH
%left MULOP WRAPMULOP DIVOP MODOP
%left OROP
%left ANDOP
%left XOROP
%nonassoc SHLOP SHROP ROTLOP ROTROP
%left POWOP WRAPPOWOP

(* Tokens that may extend an expression or start the phrase following one
   (see EXP_NO_JUXTA above); higher than every production precedence so the
   parser extends greedily. *)
%nonassoc LPAR LBRACKET TIGHT_LPAR TIGHT_LBRACKET TIGHT_HASH LCURLY ID UNDERSCORE PRIM NAT FLOAT CHAR TEXT BOOL NULL QUEST NUM_DOT_ID DISALLOWED VAR TYPE TRY TO_CANDID THROW SWITCH SHARED RETURN QUERY PERSISTENT OBJECT NOT MODULE MIXIN LOOP LET LABEL INCLUDE IGNORE IF FUNC FROM_CANDID FOR DO DEBUG_SHOW DEBUG CONTINUE COMPOSITE CLASS BREAK AWAITSTAR AWAITQUEST AWAIT ASYNCSTAR ASYNC ASSERT ACTOR PLUSASSIGN MINUSASSIGN XORASSIGN

%type<Mo_def.Syntax.exp> exp(ob, ob, exp_cont) exp_nullary(ob) exp_plain exp_obj exp_nest(ob, exp_cont) exp_nest(bl, exp_cont_tight)
%type<Mo_def.Syntax.exp * bool> exp_arg(ob) exp_arg(bl)
%type<Mo_def.Syntax.typ_item> typ_item
%type<Mo_def.Syntax.typ> typ_un typ_nullary typ typ_pre typ_nobin
%type<Mo_def.Syntax.vis> vis
%type<Mo_def.Syntax.typ_tag> typ_tag
%type<Mo_def.Syntax.typ_tag list> typ_variant
%type<Mo_def.Syntax.typ_field> typ_field
%type<Mo_def.Syntax.typ_bind> typ_bind
%type<Mo_def.Syntax.typ list> typ_args
%type<region -> Mo_def.Syntax.pat> pat_opt
%type<Mo_def.Syntax.typ_tag list> seplist1(typ_tag,semicolon) seplist(typ_tag,semicolon)
%type<Mo_def.Syntax.typ_item list> seplist(typ_item,COMMA)
%type<Mo_def.Syntax.typ_field list> typ_obj seplist(typ_field,semicolon)
%type<Mo_def.Syntax.typ_bind list> seplist(typ_bind,COMMA)
%type<Mo_def.Syntax.typ list> seplist(typ,COMMA)
%type<Mo_def.Syntax.pat_field list> seplist(pat_field,semicolon)
%type<Mo_def.Syntax.pat list> seplist(pat_bin,COMMA)
%type<Mo_def.Syntax.dec list> seplist(imp,semicolon) seplist(imp,SEMICOLON) seplist(dec,semicolon) seplist(dec,SEMICOLON)
%type<Mo_def.Syntax.exp list> seplist(exp_nonvar(ob, ob, exp_cont),COMMA) seplist(exp(ob, ob, exp_cont),COMMA)
%type<Mo_def.Syntax.exp_field list> seplist1(exp_field,semicolon) seplist(exp_field,semicolon)
%type<Mo_def.Syntax.exp list> separated_nonempty_list(AND, exp_post(ob, ob, exp_cont))
%type<Mo_def.Syntax.dec_field list> seplist(dec_field,semicolon) obj_body
%type<Mo_def.Syntax.case list> cases
%type<Mo_def.Syntax.typ option> annot_opt
%type<Mo_def.Syntax.path> path
%type<Mo_def.Syntax.pat> pat pat_un pat_plain pat_nullary pat_bin case_pat pat_paren
%type<Mo_def.Syntax.pat_field> pat_field
%type<Mo_def.Syntax.typ list option> option(typ_args)
%type<unit option> option(EQ)
%type<Mo_def.Syntax.exp> exp_un(ob, ob, exp_cont) exp_un(bl, ob, exp_cont) exp_un(bl, bl, exp_cont_tight) exp_post(ob, ob, exp_cont) exp_post(bl, ob, exp_cont) exp_post(bl, bl, exp_cont_tight) exp_nullary(bl) exp_nonvar(ob, ob, exp_cont) exp_nonvar(bl, ob, exp_cont) exp_nonvar(bl, bl, exp_cont_tight) exp_nondec(ob, ob, exp_cont) exp_nondec(bl, ob, exp_cont) exp_nondec(bl, bl, exp_cont_tight) block exp_bin(ob, ob, exp_cont) exp_bin(bl, ob, exp_cont) exp_bin(bl, bl, exp_cont_tight) exp(bl, ob, exp_cont) exp(bl, bl, exp_cont_tight)
%type<Mo_def.Syntax.exp -> region -> Mo_def.Syntax.exp> exp_cont exp_cont_tight
%type<bool * Mo_def.Syntax.exp> func_body(ob, exp_cont) func_body(bl, exp_cont_tight)
%type<Mo_def.Syntax.lit> lit
%type<Mo_def.Syntax.dec> dec imp dec_var(ob, exp_cont) dec_var(bl, exp_cont_tight) dec_nonvar(ob, exp_cont) dec_nonvar(bl, exp_cont_tight)
%type<Mo_def.Syntax.exp_field> exp_field
%type<Mo_def.Syntax.dec_field> dec_field
%type<Mo_def.Syntax.id * Mo_def.Syntax.dec_field list> class_body
%type<Mo_def.Syntax.case> catch(ob, exp_cont) catch(bl, exp_cont_tight) case
%type<Mo_def.Syntax.exp> bl ob
%type<Mo_def.Syntax.dec list> import_list
%type<Mo_def.Syntax.inst> inst
%type<Mo_def.Syntax.stab option> stab

%type<Mo_def.Syntax.dec> typ_dec
%type<Mo_def.Syntax.dec list> seplist(typ_dec,semicolon)
%type<Mo_def.Syntax.typ_field list> seplist(stab_field,semicolon)
%type<Mo_def.Syntax.typ_field> stab_field

(* recovery comment: force recovery to emit less tokens *)
%[@recover.default_cost_of_symbol     1000]
%[@recover.default_cost_of_production 1]

%[@recover.prelude
    open Mo_def.Syntax

    (* mk_stub_expr loc = VarE ("__error_recovery_var__" @~ loc) @? loc *)
    let mk_stub_expr loc = LoopE (BlockE [] @? loc, None, new_loop_flags ()) @? loc
 ]

%type<unit> start
%start<string -> Mo_def.Syntax.prog> parse_prog
%start<string -> Mo_def.Syntax.prog> parse_prog_interactive
%start<unit> parse_module_header (* Result passed via the Parser_lib.Imports exception *)
%start<string -> Mo_def.Syntax.stab_sig> parse_stab_sig
%on_error_reduce exp_bin(ob, ob, exp_cont) exp_bin(bl, ob, exp_cont) exp_bin(bl, bl, exp_cont_tight) exp_nondec(ob, ob, exp_cont) exp_nondec(bl, ob, exp_cont) exp_nondec(bl, bl, exp_cont_tight)
%%


(* Helpers *)

(* recovery comment: force to insert ";" rather immediate reduction *)
seplist(X, SEP) :
  | (* empty *) { [] }
  | x=X { [x] } [@recover.cost inf]
  | x=X SEP xs=seplist(X, SEP) { x::xs }

seplist1(X, SEP) :
  | x=X { [x] }
  | x=X SEP xs=seplist(X, SEP) { x::xs }


(* Basics *)

%inline semicolon :
  | SEMICOLON
  | SEMICOLON_EOL { () }

(* Most positions do not care whether `(`/`[` is preceded by whitespace *)
%inline lpar :
  | LPAR
  | TIGHT_LPAR { () }

%inline lbracket :
  | LBRACKET
  | TIGHT_LBRACKET { () }

%inline hash :
  | HASH
  | TIGHT_HASH { () }

%inline id :
  | id=ID { id @@ at $sloc }

%inline implicit :
  | IMPLICIT { "implicit" @@ at $sloc }

%inline typ_id :
  | id=ID { id @= at $sloc }

%inline id_opt :
  | id=id { fun _ _ -> true, id }
  | (* empty *) { fun sort sloc -> false, anon_id sort (at sloc) @@ at sloc }

%inline var_opt :
  | (* empty *) { Const @@ no_region }
  | VAR { Var @@ at $sloc }

%inline typ_obj_sort :
  | OBJECT { Type.Object @@ at $sloc }
  | ACTOR { Type.Actor @@ at $sloc }
  | MODULE {Type.Module @@ at $sloc }

%inline obj_sort :
  | OBJECT { (persistent false no_region, Type.Object @@ at $sloc) }
  | po=persistent ACTOR { (po, Type.Actor @@ at $sloc) }
  | MODULE { (persistent false no_region, Type.Module @@ at $sloc) }

%inline obj_sort_opt :
  | os=obj_sort { os }
  | (* empty *) {
      (persistent (!Flags.actors = Flags.DefaultPersistentActors) no_region, Type.Object @@ no_region)
    }

%inline query:
  | QUERY { Type.Query }
  | COMPOSITE QUERY { Type.Composite }

%inline func_sort_opt :
  | (* empty *) { Type.Local @@ no_region }
  | SHARED qo=query? { Type.Shared (Lib.Option.get qo Type.Write) @@ at $sloc }
  | q=query { Type.Shared q @@ at $sloc }

%inline shared_pat_opt :
  | (* empty *) { Type.Local @@ no_region }
  | SHARED qo=query? op=pat_opt { Type.Shared (Lib.Option.get qo Type.Write, op (at $sloc)) @@ at $sloc }
  | q=query op=pat_opt { Type.Shared (q, op (at $sloc)) @@ at $sloc }


(* Paths *)

path :
  | x=id
    { IdH x @! at $sloc }
  | p=path DOT x=id
    { DotH (p, x) @! at $sloc }

typ_path :
  | x=id
    { IdH x @= at $sloc }
  | p=path DOT x=id
    { DotH (p, x) @= at $sloc }


(* Types *)

typ_obj :
  | LCURLY tfs=seplist(typ_field, semicolon) RCURLY
    { tfs }

typ_variant :
  | LCURLY HASH RCURLY
    { [] }
  | LCURLY tfs=seplist1(typ_tag, semicolon) RCURLY
    { tfs }

typ_nullary :
  | lpar ts=seplist(typ_item, COMMA) RPAR
    { (match ts with
       | [(Some id, t)] -> NamedT(id, t)
       | [(None, t)] -> ParT t
       | _ -> TupT(ts)) @! at $sloc }
  | p=typ_path tso=typ_args?
    { PathT(p, Lib.Option.get tso []) @! at $sloc }
  | lbracket m=var_opt t=typ RBRACKET
    { ArrayT(m, t) @! at $sloc }
  | tfs=typ_obj
    { ObjT(Type.Object @@ at $sloc, tfs) @! at $sloc }
  | tfs=typ_variant
    { VariantT tfs @! at $sloc }

typ_un :
  | t=typ_nullary
    { t }
  | QUEST t=typ_un
    { OptT(t) @! at $sloc }
  | WEAK t=typ_un
    { WeakT(t) @! at $sloc }

typ_pre :
  | t=typ_un
    { t }
  | PRIM s=TEXT
    { PrimT(s) @! at $sloc }
  | ASYNC t=typ_pre
    { AsyncT(Type.Fut, scopeT (at $sloc), t) @! at $sloc }
  | ASYNCSTAR t=typ_pre
    { AsyncT(Type.Cmp, scopeT (at $sloc), t) @! at $sloc }
  | s=typ_obj_sort tfs=typ_obj
    { let tfs' =
        if s.it = Type.Actor then List.map share_typfield tfs else tfs
      in ObjT(s, tfs') @! at $sloc }

typ_nobin :
  | t=typ_pre
    { t }
  | s=func_sort_opt tps=typ_params_opt t1=typ_un ARROW t2=typ_nobin
    { funcT(s, tps, t1, t2) @! at $sloc }

typ :
  | t=typ_nobin
    { t }
  | t1=typ AND t2=typ
    { AndT(t1, t2) @! at $sloc }
  | t1=typ OR t2=typ
    { OrT(t1, t2) @! at $sloc }

typ_item :
  | i=implicit COLON t = typ { Some i, t }
  | i=id COLON t=typ { Some i, t }
  | t=typ { None, t }

typ_args :
  | LT ts=seplist(typ, COMMA) GT { ts }

(* Note: [inst] is deliberately non-nullable; calls without instantiation have
   their own productions (with [no_inst]). A nullable [inst] would force a
   reduce decision before the argument is seen, which clashes (reduce/reduce,
   unresolvable by precedence) with ending an unparenthesized `if`/`while`/
   `switch` scrutinee. *)
inst :
  | LT ts=seplist(typ, COMMA) GT
    { { it = Some (false, ts); at = at $sloc; note = [] } }
  | LT SYSTEM ts=preceded(COMMA, typ)* GT
    { { it = Some (true, ts); at = at $sloc; note = [] } }

%inline type_typ_params_opt :
  | (* empty *) { [] }
  | LT ts=seplist(typ_bind, COMMA) GT { ts }

%inline typ_params_opt :
  | ts=type_typ_params_opt { ts }
  | LT SYSTEM ts=preceded(COMMA, typ_bind)* GT { ensure_scope_bind "" ts }

typ_field :
  | TYPE c=typ_id  tps=type_typ_params_opt EQ t=typ
    { TypF (c, tps, t) @@ at $sloc }
  | mut=var_opt x=id COLON t=typ
    { ValF (x, t, mut) @@ at $sloc }
  | x=id tps=typ_params_opt t1=typ_nullary COLON t2=typ
    { let t = funcT(Type.Local @@ no_region, tps, t1, t2)
              @! span x.at t2.at in
      ValF (x, t, Const @@ no_region) @@ at $sloc }

typ_tag :
  | hash x=id t=annot_opt
    { {tag = x; typ = Lib.Option.get t (TupT [] @! at $sloc)} @@ at $sloc }

typ_bind :
  | x=id SUB t=typ
    { {var = x; sort = Type.Type @@ no_region; bound = t} @= at $sloc }
  | x=id
    { {var = x; sort = Type.Type @@ no_region; bound = PrimT "Any" @! at $sloc} @= at $sloc }

annot_opt :
  | COLON t=typ { Some t }
  | (* empty *) { None }

%inline system_opt :
  | (* Empty *) { false }
  | LT SYSTEM GT { true }

(* Expressions *)

lit :
  | NULL { NullLit }
  | b=BOOL { BoolLit b }
  | s=NAT { PreLit (s, Type.Nat) }
  | s=FLOAT { PreLit (s, Type.Float) }
  | c=CHAR { CharLit c }
  | t=TEXT { PreLit (t, Type.Text) }

%inline unop :
  | ADDOP { PosOp }
  | SUBOP { NegOp }
  | XOROP { NotOp }

%inline binop :
  | ADDOP { AddOp }
  | SUBOP { SubOp }
  | MULOP { MulOp }
  | DIVOP { DivOp }
  | MODOP { ModOp }
  | POWOP { PowOp }
  | WRAPADDOP { WAddOp }
  | WRAPSUBOP { WSubOp }
  | WRAPMULOP { WMulOp }
  | WRAPPOWOP { WPowOp }
  | ANDOP { AndOp }
  | OROP  { OrOp }
  | XOROP { XorOp }
  | SHLOP { ShLOp }
  | SHROP { ShROp }
  | ROTLOP { RotLOp }
  | ROTROP { RotROp }
  | HASH { CatOp }

%inline relop :
  | EQOP  { EqOp }
  | NEQOP { NeqOp }
  | LTOP  { LtOp }
  | LEOP  { LeOp }
  | GTOP  { GtOp }
  | GEOP  { GeOp }

%inline unassign :
  | PLUSASSIGN { PosOp }
  | MINUSASSIGN { NegOp }
  | XORASSIGN { NotOp }

%inline binassign :
  | PLUSASSIGN { AddOp }
  | MINUSASSIGN { SubOp }
  | MULASSIGN { MulOp }
  | DIVASSIGN { DivOp }
  | MODASSIGN { ModOp }
  | POWASSIGN { PowOp }
  | WRAPADDASSIGN { WAddOp }
  | WRAPSUBASSIGN { WSubOp }
  | WRAPMULASSIGN { WMulOp }
  | WRAPPOWASSIGN { WPowOp }
  | ANDASSIGN { AndOp }
  | ORASSIGN { OrOp }
  | XORASSIGN { XorOp }
  | SHLASSIGN { ShLOp }
  | SHRASSIGN { ShROp }
  | ROTLASSIGN { RotLOp }
  | ROTRASSIGN { RotROp }
  | CATASSIGN { CatOp }


bl : DISALLOWED { PrimE("dummy") @? at $sloc }
%public ob : e=exp_obj { e }

%inline parenthetical:
  | lpar base=exp_post(ob, ob, exp_cont)? WITH fs=seplist(exp_field, semicolon) RPAR
    { Some (ObjE (Option.(to_list base), fs) @? at $sloc) }

%inline parenthetical_opt :
  | p=parenthetical { p }
  | (*empty*) { None }

exp_obj :
  | LCURLY efs=seplist(exp_field, semicolon) RCURLY
    { ObjE ([], efs) @? at $sloc }
  | LCURLY base=exp_post(ob, ob, exp_cont) AND bases=separated_nonempty_list(AND, exp_post(ob, ob, exp_cont)) RCURLY
    { ObjE (base :: bases, []) @? at $sloc }
  | LCURLY bases=separated_nonempty_list(AND, exp_post(ob, ob, exp_cont)) WITH efs=seplist1(exp_field, semicolon) RCURLY
    { ObjE (bases, efs) @? at $sloc }

exp_plain :
  | l=lit
    { LitE(ref l) @? at $sloc }
  | lpar es=seplist(exp(ob, ob, exp_cont), COMMA) RPAR
    { match es with [e] -> e | _ -> TupE(es) @? at $sloc }

(* recovery comment: force to emit special variable instead of "_" to filter spurious errors *)
exp_nullary [@recover.expr mk_stub_expr loc] (B) :
  | e=B
  | e=exp_plain
    { e }
  | x=id
    { VarE (x.it @~ x.at) @? at $sloc }
  | PRIM s=TEXT
    { PrimE(s) @? at $sloc }
  | UNDERSCORE
    { VarE ("_" @~ at $sloc) @? at $sloc }

(* The expression grammar is parameterized over two modes:
   - B ("begin") controls whether the expression may START with `{`:
     under `ob` a leading `{` is a record literal, under `bl` it is not
     part of the expression at all (statement position: a leading `{` is
     a block; scrutinee position: it opens the construct's body).
   - R ("rest") controls whether a `{` may CONTINUE the expression
     anywhere later (record call argument, variant payload, operand of a
     nested operator). `if`/`while` conditions and `switch` scrutinees
     are parsed as exp(bl, bl, exp_cont_tight), so that the `{` following them always
     belongs to the construct (the Rust rule for struct literals);
     parenthesized subexpressions reset to (ob, ob). *)

exp_arg(B):
  | e=B { e, false }
  | l=lit
    { LitE(ref l) @? at $sloc, false }
  | lpar es=seplist(exp(ob, ob, exp_cont), COMMA) RPAR
    { match es with [e] -> e, true | _ -> TupE(es) @? at $sloc, false }
  | x=id
    { VarE (x.it @~ x.at) @? at $sloc, false }
  | PRIM s=TEXT
    { PrimE(s) @? at $sloc, false }
  | UNDERSCORE
    { VarE ("_" @~ at $sloc) @? at $sloc, false }


exp_post(B, R, L) :
  | e=exp_nullary(B)
    { e }
  | lbracket m=var_opt es=seplist(exp_nonvar(ob, ob, exp_cont), COMMA) RBRACKET
    { ArrayE(m, es) @? at $sloc }
  | e1=exp_post(B, R, L) c=L
    { c e1 (at $sloc) }
  | e=exp_post(B, R, L) s=DOT_NUM
    { ProjE (e, int_of_string s) @? at $sloc }
  | e=exp_post(B, R, L) DOT x=id
    { DotE(e, x, ref None) @? at $sloc }
  | nid = NUM_DOT_ID
    { let (num, id) = nid in
      let {left; right} = at $sloc in
      let e =
	LitE(ref (PreLit (num, Type.Nat))) @?
	{ left;
	  right = { right with column = left.column + String.length num }}
      in
      let x =
	id @@
	{ left = { left with column = right.column - String.length id };
	  right } in
      DotE(e, x, ref None) @? at $sloc
    }
  | e1=exp_post(B, R, L) inst=inst e2=exp_arg(R)
    {
      let e2, sugar = e2 in
      CallE(None, e1, inst, (sugar, ref e2)) @? at $sloc
    }
  | e1=exp_post(B, R, L) BANG
    { BangE(e1) @? at $sloc }
  | lpar SYSTEM e1=exp_post(B, R, L) DOT x=id RPAR
    { DotE(
        DotE(e1, "system" @@ at ($startpos($1),$endpos($1)), ref None) @? at $sloc,
        x, ref None) @? at $sloc }

(* Postfix continuations (call argument or indexing), as functions from the
   expression they extend (and the full source region) to the extended
   expression. `exp_cont_tight` admits only a `(`/`[` NOT preceded by
   whitespace; it is the continuation mode of scrutinee/condition position,
   where a spaced `(`/`[` (or any other juxtaposed atom) instead starts the
   body following the scrutinee. Everywhere else, `exp_cont` also admits the
   spaced forms and juxtaposed atomic arguments. *)
exp_cont_tight :
  | TIGHT_LPAR es=seplist(exp(ob, ob, exp_cont), COMMA) RPAR
    { let e2, sugar =
        match es with [e] -> e, true | _ -> TupE(es) @? at $sloc, false in
      fun e1 at -> CallE(None, e1, no_inst (), (sugar, ref e2)) @? at }
  | TIGHT_LBRACKET e2=exp(ob, ob, exp_cont) RBRACKET
    { fun e1 at -> IdxE(e1, e2) @? at }

exp_cont :
  | c=exp_cont_tight
    { c }
  | LPAR es=seplist(exp(ob, ob, exp_cont), COMMA) RPAR
    { let e2, sugar =
        match es with [e] -> e, true | _ -> TupE(es) @? at $sloc, false in
      fun e1 at -> CallE(None, e1, no_inst (), (sugar, ref e2)) @? at }
  | LBRACKET e2=exp(ob, ob, exp_cont) RBRACKET
    { fun e1 at -> IdxE(e1, e2) @? at }
  | e2=ob
    { fun e1 at -> CallE(None, e1, no_inst (), (false, ref e2)) @? at }
  | l=lit
    { let e2 = LitE(ref l) @? at $sloc in
      fun e1 at -> CallE(None, e1, no_inst (), (false, ref e2)) @? at }
  | x=id
    { let e2 = VarE (x.it @~ x.at) @? at $sloc in
      fun e1 at -> CallE(None, e1, no_inst (), (false, ref e2)) @? at }
  | PRIM s=TEXT
    { let e2 = PrimE(s) @? at $sloc in
      fun e1 at -> CallE(None, e1, no_inst (), (false, ref e2)) @? at }
  | UNDERSCORE
    { let e2 = VarE ("_" @~ at $sloc) @? at $sloc in
      fun e1 at -> CallE(None, e1, no_inst (), (false, ref e2)) @? at }

exp_un(B, R, L) :
  | e=exp_post(B, R, L) %prec EXP_NO_JUXTA
    { e }
  | par=parenthetical e=exp_post(B, R, L) %prec EXP_NO_JUXTA
     { match e.it with
       | CallE (None, e1, inst, args) ->
         CallE (par, e1, inst, args) @? at $sloc
       | _ ->
         syntax_error (at $sloc) "M0210"
           "misplaced parenthetical note: it must precede a function call";
         e }
  | hash x=id %prec EXP_NO_JUXTA
    { TagE (x, TupE([]) @? at $sloc) @? at $sloc }
  | hash x=id e=exp_nullary(R)
    { TagE (x, e) @? at $sloc }
  | QUEST e=exp_un(R, R, L)
    { OptE(e) @? at $sloc }
  | op=unop e=exp_un(R, R, L)
    { match op, e.it with
      | (PosOp | NegOp), LitE {contents = PreLit (s, (Type.(Nat | Float) as typ))} ->
        let signed = match op with NegOp -> "-" ^ s | _ -> "+" ^ s in
        LitE(ref (PreLit (signed, Type.(if typ = Nat then Int else typ)))) @? at $sloc
      | _ -> UnE(ref Type.Pre, op, e) @? at $sloc
    }
  | op=unassign e=exp_un(R, R, L)
    { assign_op e (fun e' -> UnE(ref Type.Pre, op, e') @? at $sloc) (at $sloc) }
  | ACTOR e=exp_plain
    { ActorUrlE e @? at $sloc }
  | NOT e=exp_un(R, R, L)
    { NotE e @? at $sloc }
  | DEBUG_SHOW e=exp_un(R, R, L)
    { ShowE (ref Type.Pre, e) @? at $sloc }
  | TO_CANDID lpar es=seplist(exp(ob, ob, exp_cont), COMMA) RPAR
    { ToCandidE es @? at $sloc }
  | FROM_CANDID e=exp_un(R, R, L)
    { FromCandidE e @? at $sloc }

%public exp_bin(B, R, L) :
  | e=exp_un(B, R, L)
    { e }
  | e1=exp_bin(B, R, L) op=binop e2=exp_bin(R, R, L)
    { BinE(ref Type.Pre, e1, op, e2) @? at $sloc }
  | e1=exp_bin(B, R, L) op=relop e2=exp_bin(R, R, L)
    { RelE(ref Type.Pre, e1, op, e2) @? at $sloc }
  | e1=exp_bin(B, R, L) AND e2=exp_bin(R, R, L)
    { AndE(e1, e2) @? at $sloc }
  | e1=exp_bin(B, R, L) OR e2=exp_bin(R, R, L)
    { OrE(e1, e2) @? at $sloc }
  | e=exp_bin(B, R, L) COLON t=typ_nobin
    { AnnotE(e, t) @? at $sloc }
  | e1=exp_bin(B, R, L) PIPE e2=exp_bin(R, R, L)
    { let x = "_" @@ e1.at in
      BlockE [
        LetD (VarP x @! x.at, e1, None) @? e1.at;
        ExpD e2 @? e2.at
      ] @? at $sloc }


%public exp_nondec(B, R, L) :
  | e=exp_bin(B, R, L) %prec EXP_NO_JUXTA
    { e }
  | e1=exp_bin(B, R, L) ASSIGN e2=exp(R, R, L)
    { AssignE(e1, e2) @? at $sloc}
  | e1=exp_bin(B, R, L) NULLCOALESCE e2=exp(R, R, L)
    { NullCoalesceE(e1, e2) @? at $sloc }
  | e1=exp_bin(B, R, L) op=binassign e2=exp(R, R, L)
    { assign_op e1 (fun e1' -> BinE(ref Type.Pre, e1', op, e2) @? at $sloc) (at $sloc) }
  | RETURN %prec RETURN_NO_ARG
    { RetE(TupE([]) @? at $sloc) @? at $sloc }
  | RETURN e=exp(R, R, L)
    { RetE(e) @? at $sloc }
  | par=parenthetical_opt ASYNC e=exp_nest(R, L)
    { AsyncE(par, Type.Fut, scope_bind (anon_id "async" (at $sloc)) (at $sloc), e) @? at $sloc }
  | ASYNCSTAR e=exp_nest(R, L)
    { AsyncE(None, Type.Cmp, scope_bind (anon_id "async*" (at $sloc)) (at $sloc), e) @? at $sloc }
  | AWAIT e=exp_nest(R, L)
    { AwaitE(Type.AwaitFut false, e) @? at $sloc }
  | AWAITQUEST e=exp_nest(R, L)
    { AwaitE(Type.AwaitFut true, e) @? at $sloc }
  | AWAITSTAR e=exp_nest(R, L)
    { AwaitE(Type.AwaitCmp, e) @? at $sloc }
  | ASSERT e=exp_nest(R, L)
    { AssertE(Runtime, e) @? at $sloc }
  | LABEL x=id rt=annot_opt e=exp_nest(R, L)
    { let x' = ("continue " ^ x.it) @@ x.at in
      let unit () = TupT [] @! at $sloc in
      let e' =
        match e.it with
        | WhileE (e1, e2, flags) -> WhileE (e1, LabelE (x', unit (), e2) @? e2.at, flags) @? e.at
        | LoopE (e1, eo, flags) -> LoopE (LabelE (x', unit (), e1) @? e1.at, eo, flags) @? e.at
        | ForE (p, e1, e2, flags) -> ForE (p, e1, LabelE (x', unit (), e2) @? e2.at, flags) @? e.at
        | _ -> e
      in
      LabelE(x, Lib.Option.get rt (unit ()), e') @? at $sloc }
  | BREAK x=id %prec EXP_NO_JUXTA
    { let e = TupE([]) @? at $sloc in
      BreakE(Break, Some x, e) @? at $sloc }
  | BREAK x=id e=exp_nullary(R)
    { BreakE(Break, Some x, e) @? at $sloc }
  | BREAK %prec EXP_NO_JUXTA
    { let e = TupE([]) @? at $sloc in
      BreakE(Break, None, e) @? at $sloc }
  | CONTINUE %prec EXP_NO_JUXTA
    { let e = TupE([]) @? at $sloc in
      BreakE(Continue, None, e) @? at $sloc }
  | CONTINUE x=id
    { let e = TupE([]) @? at $sloc in
      let x' = ("continue " ^ x.it) @@ x.at in
      BreakE(Continue, Some x', e) @? at $sloc }
  | DEBUG e=exp_nest(R, L)
    { DebugE(e) @? at $sloc }
  | IF b=exp(bl, bl, exp_cont_tight) e1=exp_nest(R, L) %prec IF_NO_ELSE
    { IfE(b, e1, TupE([]) @? at $sloc) @? at $sloc }
  | IF b=exp(bl, bl, exp_cont_tight) e1=exp_nest(R, L) ELSE e2=exp_nest(R, L)
    { IfE(b, e1, e2) @? at $sloc }
  | TRY e1=exp_nest(R, L) c=catch(R, L) %prec TRY_CATCH_NO_FINALLY
    { TryE(e1, [c], None) @? at $sloc }
  | TRY e1=exp_nest(R, L) c=catch(R, L) FINALLY e2=exp_nest(R, L)
    { TryE(e1, [c], Some e2) @? at $sloc }
  | TRY e1=exp_nest(R, L) FINALLY e2=exp_nest(R, L)
    { TryE(e1, [], Some e2) @? at $sloc }
(* TODO: enable multi-branch TRY (already supported by compiler)
  | TRY e=exp_nest(R, L) LCURLY cs=cases RCURLY
    { TryE(e, cs) @? at $sloc }
*)
  | THROW e=exp_nest(R, L)
    { ThrowE(e) @? at $sloc }
  | SWITCH e=exp(bl, bl, exp_cont_tight) LCURLY cs=cases RCURLY
    { SwitchE(e, cs) @? at $sloc }
  | WHILE e1=exp(bl, bl, exp_cont_tight) e2=exp_nest(R, L)
    { WhileE(e1, e2, new_loop_flags ()) @? at $sloc }
  | LOOP e=exp_nest(R, L) %prec LOOP_NO_WHILE
    { LoopE(e, None, new_loop_flags ()) @? at $sloc }
  | LOOP e1=exp_nest(R, L) WHILE e2=exp_nest(R, L)
    { LoopE(e1, Some e2, new_loop_flags ()) @? at $sloc }
  | FOR lpar p=pat IN e1=exp(ob, ob, exp_cont) RPAR e2=exp_nest(R, L)
    { ForE(p, e1, e2, new_loop_flags ()) @? at $sloc }
  | FOR p=pat IN e1=exp(bl, bl, exp_cont_tight) e2=exp_nest(R, L)
    { ForE(p, e1, e2, new_loop_flags ()) @? at $sloc }
  | IGNORE e=exp_nest(R, L)
    { IgnoreE(e) @? at $sloc }
  | DO e=block
    { e.it @? at $sloc }
  | DO QUEST e=block
    { DoOptE(e) @? at $sloc }

exp_nonvar(B, R, L) :
  | e=exp_nondec(B, R, L)
    { e }
  | d=dec_nonvar(R, L)
    { match d.it with ExpD e -> e | _ -> BlockE([d]) @? at $sloc }

(* recovery comment: force to emit special variable rather than "return" *)
exp [@recover.expr mk_stub_expr loc] (B, R, L) :
  | e=exp_nonvar(B, R, L)
    { e }
  | d=dec_var(R, L)
    { BlockE([d]) @? at $sloc }

%public exp_nest(R, L) :
  | e=block
  | e=exp(bl, R, L)
    { e }

block :
  | LCURLY ds=seplist(dec, semicolon) RCURLY
    { BlockE(ds) @? at $sloc }

case :
  | CASE p=case_pat e=exp_nest(ob, exp_cont)
    { {pat = p; exp = e} @@ at $sloc }

(* The semicolon between cases is optional: every case starts with the
   `case` keyword, so the separator disambiguates nothing. *)
cases :
  | (* empty *) { [] }
  | c=case cs=cases { c::cs }
  | c=case semicolon cs=cases { c::cs }

catch(R, L) :
  | CATCH p=pat_nullary e=exp_nest(R, L)
    { {pat = p; exp = e} @@ at $sloc }

exp_field :
  | m=var_opt x=id t=annot_opt
    { let e = VarE (x.it @~ x.at) @? x.at in
      { mut = m; id = x; exp = annot_exp e t; } @@ at $sloc }
  | m=var_opt x=id t=annot_opt EQ e=exp(ob, ob, exp_cont)
    { { mut = m; id = x; exp = annot_exp e t; } @@ at $sloc }

dec_field :
  | v=vis s=stab d=dec
    { {dec = d; vis = v; stab = s} @@ at $sloc }

vis :
  | (* empty *) { Private @@ no_region }
  | PRIVATE { Private @@ at $sloc }
  | PUBLIC {
    let at = at $sloc in
    let trivia = Trivia.find_trivia !triv_table at in
    let depr = Trivia.deprecated_of_trivia_info trivia in
    Public depr @@ at }
  | SYSTEM { System @@ at $sloc }

stab :
  | (* empty *) { None }
  | FLEXIBLE { Some (Flexible @@ at $sloc) }
  | STABLE { Some ((Stable (ref None)) @@ at $sloc) }
  | TRANSIENT { Some (Flexible @@ at $sloc) }

%inline persistent :
  | (* empty *) { persistent (!Flags.actors = Flags.DefaultPersistentActors) no_region }
  | PERSISTENT { persistent true (at $sloc) }

(* Patterns *)

pat_plain :
  | UNDERSCORE
    { WildP @! at $sloc }
  | x=id
    { VarP(x) @! at $sloc }
  | l=lit
    { LitP(ref l) @! at $sloc }
  | lpar ps=seplist(pat_bin, COMMA) RPAR
    { (match ps with [p] -> ParP(p) | _ -> TupP(ps)) @! at $sloc }

pat_nullary :
  | p=pat_plain
    { p }
  | LCURLY fps=seplist(pat_field, semicolon) RCURLY
    { ObjP(fps) @! at $sloc }

pat_un :
  | p=pat_nullary
    { p }
  | hash x=id %prec EXP_NO_JUXTA
    { TagP(x, TupP [] @! at $sloc) @! at $sloc }
  | hash x=id p=pat_nullary
    { TagP(x, p) @! at $sloc }
  | QUEST p=pat_un
    { OptP(p) @! at $sloc }
  | op=unop l=lit
    { match op, l with
      | (PosOp | NegOp), PreLit (s, (Type.(Nat | Float) as typ)) ->
        let signed = match op with NegOp -> "-" ^ s | _ -> "+" ^ s in
        LitP(ref (PreLit (signed, Type.(if typ = Nat then Int else typ)))) @! at $sloc
      | _ -> SignP(op, ref l) @! at $sloc
    }

pat_bin :
  | p=pat_un
    { p }
  | p1=pat_bin OR p2=pat_bin
    { AltP(p1, p2) @! at $sloc }
  | p1=pat_bin AND p2=pat_bin
    { AndP(p1, p2) @! at $sloc }
  | p=pat_bin COLON t=typ
    { AnnotP(p, t) @! at $sloc }

pat :
  | p=pat_bin
    { p }

(* Deliberately just the parenthesized form of pat_plain: a variant payload in
   an unparenthesized case pattern must itself be parenthesized, so that in
   `case #tag { ... }` the braces are unambiguously the case body. *)
pat_paren :
  | lpar ps=seplist(pat_bin, COMMA) RPAR
    { (match ps with [p] -> ParP(p) | _ -> TupP(ps)) @! at $sloc }

(* Case patterns whose extent is deterministic without parentheses:
   `case null`, `case 0`, `case ?p`, `case #tag`, `case #tag(p)`.
   Anything else still needs parentheses around the whole pattern. *)
case_pat :
  | p=pat_nullary
    { p }
  | hash x=id %prec EXP_NO_JUXTA
    { TagP(x, TupP [] @! at $sloc) @! at $sloc }
  | hash x=id p=pat_paren
    { TagP(x, p) @! at $sloc }
  | QUEST p=case_pat
    { OptP(p) @! at $sloc }
  | op=unop l=lit
    { match op, l with
      | (PosOp | NegOp), PreLit (s, (Type.(Nat | Float) as typ)) ->
        let signed = match op with NegOp -> "-" ^ s | _ -> "+" ^ s in
        LitP(ref (PreLit (signed, Type.(if typ = Nat then Int else typ)))) @! at $sloc
      | _ -> SignP(op, ref l) @! at $sloc
    }

pat_field :
  | x=id t=annot_opt
    { ValPF(x, annot_pat (VarP x @! x.at) t) @@ at $sloc }
  | x=id t=annot_opt EQ p=pat
    { ValPF(x, annot_pat p t) @@ at $sloc }
  | TYPE x=typ_id
    { TypPF(x) @@ at $sloc }

pat_opt :
  | p=pat_plain
    { fun sloc -> p }
  | (* empty *)
    { fun sloc -> WildP @! sloc }

func_pat :
  | xf=id_opt ts=typ_params_opt p=pat_plain { (xf, ts, p) }

(* Declarations *)

dec_var(R, L) :
  | VAR x=id t=annot_opt EQ e=exp(R, R, L)
    { VarD(x, annot_exp e t) @? at $sloc }
  | VAR x=id COLON t=typ
    (* No initializer - use PrimE "_" : None as placeholder *)
    (* Type checker will verify this is only allowed for stable variables with --enhanced-migration *)
    { let init_exp = PrimE "_" @? at $sloc in
      VarD(x, annot_exp init_exp (Some t)) @? at $sloc }

dec_nonvar(R, L) :
  | LET p=pat EQ e=exp(R, R, L)
    { let p', e' = normalize_let p e in
      LetD (p', e', None) @? at $sloc }
  | LET p=pat
    (* because of shift/reduce conflict with LET id COLON typ,
       we parse a full pat but reject during typing *)
    { let p', e' = normalize_let p (PrimE "_" @? at $sloc) in
      LetD (p', e', None) @? at $sloc }
  | TYPE x=typ_id tps=type_typ_params_opt EQ t=typ
    { TypD(x, tps, t) @? at $sloc }
  | sp=shared_pat_opt FUNC
      xf_tps_p=func_pat t=annot_opt fb=func_body(R, L)
    { (* This is a hack to support local func declarations that return a computed async.
         These should be defined using RHS syntax EQ e to avoid the implicit AsyncE introduction
         around bodies declared as blocks *)
      let xf, tps, p = xf_tps_p in
      let named, x = xf "func" $sloc in
      let is_sugar, e = desugar_func_body sp x t fb in
      let_or_exp named x (func_exp x.it sp tps p t is_sugar e) (at $sloc) }
  | eo=parenthetical_opt mk_d=obj_or_class_dec  { mk_d eo }
  | MIXIN system=system_opt p=pat_plain dfs=obj_body {
     let dfs = List.map (share_dec_field (fun () -> Stable (ref None) @@ no_region)) dfs in
     MixinD(system, p, dfs) @? at $sloc
  }
  | INCLUDE x=id system=system_opt e=exp(R, R, L) { IncludeD(x, system, e, ref None) @? at $sloc }

obj_or_class_dec :
  | ds=obj_sort xf=id_opt t=annot_opt EQ? efs=obj_body
    { fun eo ->
      let (persistent, s) = ds in
      let sort = Type.(match s.it with
                       | Actor -> "actor" | Module -> "module" | Object -> "object"
                       | _ -> assert false) in
      let named, x = xf sort $sloc in
      let e =
        if s.it = Type.Actor then
          let default_stab () = (if persistent.it then (Stable (ref None)) else Flexible) @@ no_region in
          let id = if named then Some x else None in
          AwaitE
            (Type.AwaitFut false,
             AsyncE(None, Type.Fut, scope_bind (anon_id "async" (at $sloc)) (at $sloc),
                    objblock eo { s with note = persistent } id t (List.map (share_dec_field default_stab) efs) @? at $sloc)
             @? at $sloc) @? at $sloc
        else objblock eo { s with note = persistent } None t efs @? at $sloc
      in
      let_or_exp named x e.it e.at }
  | sp=shared_pat_opt ds=obj_sort_opt CLASS
      xf_tps_p=func_pat t=annot_opt  cb=class_body
    { fun eo ->
      let (persistent, s) = ds in
      let xf, tps, p = xf_tps_p in
      let (_, id) = xf "class" $sloc in
      let cid = id.it @= id.at in
      let x, dfs = cb in
      let dfs', tps', t' =
       if s.it = Type.Actor then
         let default_stab () = (if persistent.it then Stable (ref None) else Flexible) @@ no_region in
          (List.map (share_dec_field default_stab) dfs,
           ensure_scope_bind "" tps,
           (* Not declared async: insert AsyncT but deprecate in typing *)
           ensure_async_typ t)
        else (dfs, tps, t)
      in
      ClassD(eo, sp, {s with note = persistent}, cid, tps', p, t', x, dfs') @? at $sloc }

dec :
  | d=dec_var(ob, exp_cont)
    { d }
  | d=dec_nonvar(ob, exp_cont)
    { d }
  | e=exp_nondec(ob, ob, exp_cont)
    { ExpD e @? at $sloc }
  | LET p=pat EQ e=exp(ob, ob, exp_cont) ELSE fail=exp_nest(ob, exp_cont)
    { let p', e' = normalize_let p e in
      LetD (p', e', Some fail) @? at $sloc }
  (* error production: `x = e` in declaration position is a common mis-spelling
     of a record field (`{ ... }` is a block here) or of `let`/`:=` *)
  | x=id EQ e=exp(ob, ob, exp_cont)
    { syntax_error (at $sloc) "M0269"
        "a record literal is not allowed here, braces `{ ... }` enclose a block in this position; to write a record, wrap it in parentheses: `({ ... })`; to declare a variable, use `let`; to assign, use `:=`";
      let ef = { mut = Const @@ no_region; id = x; exp = e } @@ at $sloc in
      ExpD (ObjE ([], [ef]) @? at $sloc) @? at $sloc }

func_body(R, L) :
  | EQ e=exp(R, R, L) { (false, e) }
  | e=block { (true, e) }

obj_body :
  | LCURLY dfs=seplist(dec_field, semicolon) RCURLY { dfs }

class_body :
  | EQ xf=id_opt dfs=obj_body { snd (xf "object" $sloc), dfs }
  | dfs=obj_body { anon_id "object" (at $sloc) @@ at $sloc, dfs }


(* Programs *)

imp :
  | IMPORT p=pat_nullary EQ? f=TEXT
    { LetD(p, ImportE(f, ref Unresolved) @? at $sloc, None) @? at $sloc }

start : (* dummy non-terminal to satisfy ErrorReporting.ml, that requires a non-empty parse stack *)
  | (* empty *) { () }

parse_prog :
  | start is=seplist(imp, semicolon) ds=seplist(dec, semicolon) EOF
    {
      let trivia = !triv_table in
      fun filename -> { it = is @ ds; at = at $sloc; note = { filename; trivia }} }

parse_prog_interactive :
  | start is=seplist(imp, SEMICOLON) ds=seplist(dec, SEMICOLON) SEMICOLON_EOL
    {
      let trivia = !triv_table in
      fun filename -> {
        it = is @ ds;
        at = at $sloc;
        note = { filename; trivia }
      }
    }

import_list :
  | is=seplist(imp, semicolon) { raise (Imports is) }

parse_module_header :
  | start import_list EOF {}

(* stable signatures (.most files) *)

typ_dec :
  | TYPE x=typ_id tps=type_typ_params_opt EQ t=typ
    { TypD(x, tps, t) @? at $sloc }

stab_field :
  | STABLE mut=var_opt x=id COLON t=typ
    { ValF (x, t, mut) @@ at $sloc }

pre_stab_field :
  | r=req mut=var_opt x=id COLON t=typ
    { (r, ValF (x, t, mut) @@ at $sloc) }

%inline req :
  | STABLE { false @@ at $sloc }
  | IN { true @@ at $sloc }

mig_lab : t=TEXT { t @@ at $sloc }
mig_field :
  | mt=mig_lab COLON t=typ
    { {tag=mt; typ=t} @@ at $sloc }

parse_stab_sig :
  | start ds=seplist(typ_dec, semicolon) ACTOR LCURLY sfs=seplist(stab_field, semicolon) RCURLY
    { let trivia = !triv_table in
      let sigs = Single sfs in
      fun filename -> {
          it = (ds, {it = sigs; at = at $sloc; note = ()});
          at = at $sloc;
          note = { filename; trivia } }
    }
  | start ds=seplist(typ_dec, semicolon)
       ACTOR lpar LCURLY sfs_pre=seplist(pre_stab_field, semicolon) RCURLY COMMA
             LCURLY sfs_post=seplist(stab_field, semicolon) RCURLY  RPAR
    { let trivia = !triv_table in
      let sigs = PrePost(sfs_pre, sfs_post) in
      fun filename ->
        { it = (ds, {it = sigs; at = at $sloc; note = ()});
          at = at $sloc;
          note = { filename; trivia } }
    }
  | start ds=seplist(typ_dec, semicolon)
    LCURLY chain = seplist(mig_field, semicolon) RCURLY
    ACTOR LCURLY sfs_post=seplist(stab_field, semicolon) RCURLY
    { let trivia = !triv_table in
      let sigs = Multi{chain;post=sfs_post} in
      fun filename ->
        { it = (ds, {it = sigs; at = at $sloc; note = ()});
          at = at $sloc;
          note = { filename; trivia } }
    }


%%
