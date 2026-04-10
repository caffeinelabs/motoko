%{

open Syntax
open Source

module Uint32 = Lib.Uint32

(* Position handling *)

let position_to_pos position =
  (* TBR: Remove assertion once the menhir bug is fixed. *)
  assert (Obj.is_block (Obj.repr position));
  { file = position.Lexing.pos_fname;
    line = position.Lexing.pos_lnum;
    column = position.Lexing.pos_cnum - position.Lexing.pos_bol
  }

let positions_to_region position1 position2 =
  { left = position_to_pos position1;
    right = position_to_pos position2
  }

let at (startpos, endpos) = positions_to_region startpos endpos

let _anon sort at = "anon-" ^ sort ^ "-" ^ string_of_pos at.left

let prim_typs = ["nat", Nat; "nat8", Nat8; "nat16", Nat16; "nat32", Nat32; "nat64", Nat64;
                 "int", Int; "int8", Int8; "int16", Int16; "int32", Int32; "int64", Int64;
                 "float32", Float32; "float64", Float64; "bool", Bool; "text", Text;
                 "null", Null; "reserved", Reserved; "empty", Empty]
let is_prim_typs t = List.assoc_opt t prim_typs

let hash = IdlHash.idl_hash
let record_fields get_label fs =
  let open Uint32 in
  let rec go start fs =
    match fs with
    | [] -> []
    | hd :: tl ->
       let field = hd start in
       let next =
         (match (get_label field.it).it with
         | Id n -> succ n
         | Named name -> succ (hash name)
         | Unnamed n -> succ n) in
       field :: (go next tl)
  in go zero fs
%}

%token EOF

%token LPAR RPAR LCURLY RCURLY
%token ARROW MINUS
%token FUNC TYPE SERVICE IMPORT PRINCIPAL
%token SEMICOLON COMMA PERIOD COLON EQ
%token NOTCOLON EQQ NOTEQ
%token OPT VEC RECORD VARIANT BLOB ONEWAY QUERY COMPOSITE_QUERY
%token<string> NAT
%token<string> ID
%token<string> TEXT

%start<string -> Syntax.prog> parse_prog
%start<string -> Syntax.tests> parse_tests
%start<Syntax.args> parse_args

%%

(* Helpers *)

endlist(X, SEP) :
  | (* empty *) { [] }
  | x=X SEP xs=seplist(X, SEP) { x::xs }

seplist(X, SEP) :
  | (* empty *) { [] }
  | x=X { [x] }
  | x=X SEP xs=seplist(X, SEP) { x::xs }

(* Basics *)

%inline text :
 | s=TEXT
   { try ignore (Lib.Utf8.decode s); s
     with Lib.Utf8.Utf8 -> raise (ParseError (at $sloc, "Invalid UTF-8"))
   }

%inline id :
  | id=ID { id @@ at $sloc }

%inline name :
  | id=ID { id @@ at $sloc }
  | text=text { text @@ at $sloc }

(* Types *)

prim_typ :
  | x=id
    { (match is_prim_typs x.it with
         None -> VarT x
       | Some t -> PrimT t) @@ at $sloc }

ref_typ :
  | FUNC t=func_typ { t }
  | SERVICE ts=actor_typ { ServT ts @@ at $sloc }
  | PRINCIPAL { PrincipalT @@ at $sloc }

field_typ :
  | n=NAT COLON t=data_typ
    { { label = Id (Uint32.of_string n) @@ at $loc(n); typ = t } @@ at $sloc }
  | name=name COLON t=data_typ
    { { label = Named name.it @@ at $loc(name); typ = t } @@ at $sloc }

record_typ :
  | f=field_typ { fun _ -> f }
  | t=data_typ
    { fun x -> { label = Unnamed x @@ no_region; typ = t } @@ at $sloc }

variant_typ :
  | f=field_typ { f }
  | name=name
    { { label = Named name.it @@ at $loc(name); typ = PrimT Null @@ no_region } @@ at $sloc }
  | n=NAT
    { { label = Id (Uint32.of_string n) @@ at $loc(n); typ = PrimT Null @@ no_region } @@ at $sloc }

record_typs :
  | LCURLY fs=seplist(record_typ, SEMICOLON) RCURLY
    { record_fields (fun f -> f.label) fs }

variant_typs :
  | LCURLY fs=seplist(variant_typ, SEMICOLON) RCURLY { fs }

cons_typ :
  | OPT t=data_typ { OptT t @@ at $sloc }
  | VEC t=data_typ { VecT t @@ at $sloc }
  | RECORD fs=record_typs { RecordT fs @@ at $sloc }
  | VARIANT fs=variant_typs { VariantT fs @@ at $sloc }
  | BLOB { BlobT @@ at $sloc }

data_typ :
  | t=cons_typ { t }
  | t=ref_typ { t }
  | t=prim_typ { t }

typs :
  | LPAR fs=seplist(data_typ, COMMA) RPAR
    { fs }

param_typs :
  | LPAR fs=seplist(param_typ, COMMA) RPAR
    { fs }

param_typ :
  | t=data_typ { { name = None; typ = t} @@ at $sloc }
  | n=name COLON t=data_typ { { name = Some n; typ = t} @@ at $sloc }

func_mode :
  | ONEWAY { Oneway @@ at $sloc }
  | QUERY { Query @@ at $sloc }
  | COMPOSITE_QUERY { Composite @@ at $sloc }

func_modes_opt :
  | (* empty *) { [] }
  | m=func_mode ms=func_modes_opt { m::ms }

func_typ :
  | t1=param_typs ARROW t2=param_typs ms=func_modes_opt
    { FuncT(ms, t1, t2) @@ at $sloc }

meth_typ :
  | x=name COLON t=func_typ
    { { var = x; meth = t } @@ at $sloc }
  | x=name COLON id=id
    { { var = x; meth = VarT id @@ at $sloc } @@ at $sloc }

actor_typ :
  | LCURLY ds=seplist(meth_typ, SEMICOLON) RCURLY
    { ds }

(* Declarations *)

def :
  | TYPE x=id EQ t=data_typ
    { TypD(x, t) @@ at $sloc }
  (* TODO enforce all imports to go first in the type definitions  *)
  | IMPORT file=text
    { ImportD (file, ref "") @@ at $sloc }

id_opt :
  | (* empty *) { }
  | id { }

actor_class_typ :
  | args=param_typs ARROW tys=actor_typ
    { ClassT (args, ServT tys @@ at $loc(tys)) @@ at $sloc }
  | args=param_typs ARROW x=id
    { ClassT (args, VarT x @@ x.at) @@ at $sloc }

actor :
  | (* empty *) { None }
  | SERVICE id_opt COLON tys=actor_typ SEMICOLON?
    { Some (ServT tys @@ at $loc(tys)) }
  | SERVICE id_opt COLON x=id SEMICOLON?
    { Some (VarT x @@ x.at) }
  | SERVICE id_opt COLON t=actor_class_typ SEMICOLON?
    { Some t }

(* Programs *)

parse_prog :
  | ds=seplist(def, SEMICOLON) actor=actor EOF
    {
      let trivia = Trivia.PosHashtbl.create 0 in
      fun filename -> {it = {decs=ds; actor=actor}; at = at $sloc; note = {filename; trivia}} }

(* Values *)

(* parses number literals as strings, syntax coincides with motoko’s *)
(* very liberal parser, but we only care about good input  *)
num :
  | n=NAT PERIOD n2=num { n ^ "." ^ n2 }
  | n=NAT PERIOD { n ^ "." }
  | n=NAT { n }

signed_num :
  | MINUS n=num { "-" ^ n }
  | n=num { n }

value :
  | n=signed_num { NumV n @@ at $sloc }
  | OPT v=value { OptV v @@ at $sloc }
  | VEC LCURLY vs=seplist(value, SEMICOLON) RCURLY
    { VecV vs @@ at $sloc }
  | BLOB text=TEXT { BlobV text @@ at $sloc }
  | text=TEXT { TextV text @@ at $sloc }
  | RECORD LCURLY fs=seplist(field_value, SEMICOLON) RCURLY
    { RecordV (record_fields (fun f -> fst f) fs) @@ at $sloc }
  | VARIANT LCURLY f=variant_value RCURLY
    { VariantV f @@ at $sloc }
  | SERVICE s=name { ServiceV s.it @@ at $sloc }
  | FUNC s=name PERIOD m=name { FuncV (s.it, m.it) @@ at $sloc }
  | PRINCIPAL s=name { PrincipalV s.it @@ at $sloc }
  | b=id
    { match b.it with
      | "null" -> NullV @@ at $sloc
      | "true" -> BoolV true @@ at $sloc
      | "false" -> BoolV false @@ at $sloc
      | _ -> raise (ParseError (at $loc(b), b.it))
    }

variant_value :
  | n=NAT EQ v=value
    { (Id (Uint32.of_string n) @@ at $loc(n), v ) @@ at $sloc }
  | name=name EQ v=value
    { (Named name.it @@ at $loc(name), v) @@ at $sloc }
  | n=NAT
    { (Id (Uint32.of_string n) @@ at $loc(n), NullV @@ no_region) @@ at $sloc }
  | name=name
    { (Named name.it @@ at $loc(name), NullV @@ no_region) @@ at $sloc }

field_value :
  | n=NAT EQ v=value
    { fun _ -> (Id (Uint32.of_string n) @@ at $loc(n), v) @@ at $sloc }
  | name=name EQ v=value
    { fun _ -> (Named name.it @@ at $loc(name), v) @@ at $sloc }
  | v=value
    { fun n -> (Unnamed n @@ no_region, v) @@ at $sloc }

annval :
  | v=value { v }

parse_args :
  | LPAR vs=seplist(annval, COMMA) RPAR EOF { vs @@ at $sloc }


(* Test suite *)

input :
  | text=text { TextualInput text }
  | BLOB text=TEXT { BinaryInput text }

assertion :
  | input=input COLON    { ParsesAs (true,  input) }
  | input=input NOTCOLON { ParsesAs (false, input) }
  | i1=input EQQ   i2=input COLON { ParsesEqual (true,  i1, i2) }
  | i1=input NOTEQ i2=input COLON { ParsesEqual (false, i1, i2) }

test :
  | id=id assertion=assertion tys=typs desc=text?
    { if id.it <> "assert" then raise (ParseError (at $loc(id), id.it))
      else { ttyp=tys; assertion; desc } @@ at $sloc }

parse_tests :
  | tdecs=endlist(def, SEMICOLON) tests=seplist(test, SEMICOLON) EOF
    { fun filename -> {it = {tdecs; tests}; at = at $sloc; note = filename} }

%%
