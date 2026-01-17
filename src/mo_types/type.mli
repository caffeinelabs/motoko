(* Representation *)

type id = string
type lab = string
type var = string
type name = string

type control = Returns | Promises | Replies
type obj_sort = Object | Actor | Mixin | Module | Memory
type async_sort = Fut | Cmp
type await_sort = AwaitFut of bool | AwaitCmp
type shared_sort = Query | Write | Composite
type 'a shared = Local | Shared of 'a
type func_sort = shared_sort shared
type eff = Triv | Await

type prim =
  | Null
  | Bool
  | Nat
  | Nat8
  | Nat16
  | Nat32
  | Nat64
  | Int
  | Int8
  | Int16
  | Int32
  | Int64
  | Float
  | Char
  | Text
  | Blob (* IR use: Packed representation, vec u8 IDL type *)
  | Error
  | Principal
  | Region

type t = typ

and typ =
  | Var of var * int                          (* variable *)
  | Con of con * typ list                     (* constructor *)
  | Prim of prim                              (* primitive *)
  | Obj of obj_sort * field list              (* object *)
  | Variant of field list                     (* variant *)
  | Array of typ                              (* array *)
  | Opt of typ                                (* option *)
  | Tup of typ list                           (* tuple *)
  | Func of func_sort * control * bind list * typ list * typ list  (* function *)
  | Async of async_sort * scope * typ         (* future / computation *)
  | Mut of typ                                (* mutable type *)
  | Any                                       (* top *)
  | Non                                       (* bottom *)
  | Typ of con                                (* type (field of module) *)
  | Named of name * typ
  | Weak of typ                               (* weak references *)
  | Pre                                       (* pre-type *)

and scope = typ

and bind_sort = Scope | Type
and bind = {var : var; sort: bind_sort; bound : typ}

and src = {depr : string option; track_region : Source.region; region : Source.region}
and field = {lab : lab; typ : typ; src : src}

and con = kind Cons.t
and kind =
  | Def of bind list * typ
  | Abs of bind list * typ

val empty_src : src

(* Syntactic orderings *)

module Ord : sig
  type t = typ
  val compare : t -> t -> int
end

module OrdPair : sig
  type t = typ * typ
  val compare : t -> t -> int
end


(* Function sorts *)

val is_shared_sort : 'a shared -> bool


(* Shorthands *)

val unit : typ
val bool : typ
val nat : typ
val nat32 : typ
val nat64 : typ
val int : typ
val text : typ
val blob : typ
val error : typ
val char : typ
val principal : typ
val region : typ
val heartbeat_type : typ
val timer_type : typ
val global_timer_set_type : typ
val low_memory_type : typ

val sum : (lab * typ) list -> typ
val obj : obj_sort -> (lab * typ) list -> typ

val throwErrorCodes : field list
val catchErrorCodes : field list
val throw : typ
val catch : typ

val caller : typ
val ctxt: typ

val iter_obj : typ -> typ

val prim : string -> prim


(* Projection *)

val is_non : typ -> bool
val is_prim : prim -> typ -> bool
val is_obj : typ -> bool
val is_module : typ -> bool
val is_immutable_obj : typ -> bool
val is_variant : typ -> bool
val is_array : typ -> bool
val is_opt : typ -> bool
val is_tup : typ -> bool
val is_unit : typ -> bool
val is_pair : typ -> bool
val is_func : typ -> bool
val is_async : typ -> bool
val is_fut : typ -> bool
val is_cmp : typ -> bool
val is_mut : typ -> bool
val is_typ : typ -> bool
val is_con : typ -> bool
val is_var : typ -> bool

val as_prim : prim -> typ -> unit
val as_obj : typ -> obj_sort * field list
val as_variant : typ -> field list
val as_array : typ -> typ
val as_opt : typ -> typ
val as_tup : typ -> typ list
val as_unit : typ -> unit
val as_pair : typ -> typ * typ
val as_func : typ -> func_sort * control * bind list * typ list * typ list
val as_async : typ -> async_sort * typ * typ
val as_mut : typ -> typ
val as_immut : typ -> typ
val as_typ : typ -> con
val as_con : typ -> con * typ list

val as_prim_sub : prim -> typ -> unit
val as_obj_sub : string list -> typ -> obj_sort * field list
val as_variant_sub : string -> typ -> field list
val as_array_sub : typ -> typ
val as_opt_sub : typ -> typ
val as_tup_sub : int -> typ -> typ list
val as_unit_sub : typ -> unit
val as_pair_sub : typ -> typ * typ
val as_func_sub : func_sort -> int -> typ -> func_sort * bind list * typ list * typ
val as_mono_func_sub : typ -> typ * typ
val as_async_sub : async_sort -> typ -> typ -> typ * typ
val as_weak_sub : typ -> typ

(* Argument/result sequences *)

val seq : typ list -> typ
val codom : control -> (unit -> typ) -> typ list -> typ
val as_seq : typ -> typ list (* This needs to go away *)
val seq_of_tup : typ -> typ list
val arity : typ -> int


(* Fields *)

val find_val_field_opt : string -> field list -> field option
val lookup_val_field : string -> field list -> typ
val lookup_typ_field : string -> field list -> con
val lookup_val_field_opt : string -> field list -> typ option
val lookup_typ_field_opt : string -> field list -> con option

val lookup_val_deprecation : string -> field list -> string option
val lookup_typ_deprecation : string -> field list -> string option

val val_fields : field list -> field list

val compare_field : field -> field -> int
val align_fields : field list -> field list -> (field, field) Lib.these Seq.t

(* Constructors *)

val set_kind : con -> kind -> unit

module ConEnv : Env.S with type key = con
module ConSet = ConEnv.Dom


(* Sets *)

module S : Set.S with type elt = typ


(* Normalization and Classification *)

val normalize : typ -> typ
val promote : typ -> typ

val opaque : typ -> bool
val concrete : typ -> bool

type path = IdP of id | DotP of path * lab
val compare_path : path -> path -> int
val paths : path -> typ -> path ConEnv.t

val shared : typ -> bool
val find_unshared : typ -> typ option

val is_shared_func : typ -> bool
val is_local_async_func : typ -> bool

val stable : typ -> bool

val inhabited : typ -> bool
val singleton : typ -> bool

(** Determines if a type has no proper supertypes (except for [Any]). *)
val has_no_supertypes : typ -> bool
(** Determines if a type has no proper subtypes (except for [Non]). *)
val has_no_subtypes : typ -> bool
val span : typ -> int option


(* Constructor occurrences *)

val cons: typ -> ConSet.t
val cons_kind : kind -> ConSet.t
val cons_typs : typ list -> ConSet.t


(* Equivalence and Subtyping *)

type compatibility = Compatible | Incompatible of explanation
and explanation =
  | IncompatibleTypes of context * typ * typ
  | FailedPromote of typ * typ * explanation
  | MissingTag of context * desc * lab * typ
  | MissingField of context * desc * lab * typ
  | FewerItems of context * string
  | MoreItems of context * string
  | PromotionToAny of context * typ
  | IncompatiblePrims of context * typ * typ
  | IncompatibleObjSorts of context * obj_sort * obj_sort
  | IncompatibleFuncSorts of context * func_sort * func_sort
  | IncompatibleFuncControls of context * control * control
  | IncompatibleFuncs of context * typ * typ
  | IncompatibleAsyncSorts of context * async_sort * async_sort
  | IncompatibleAsyncScopes of context * typ * typ
and desc = Actual | Expected
and context_item =
  | ConsType of con
  | NamedType of name
  | StableVariable of lab
  | Field of lab
  | Bounds
  | Domain
  | CoDomain
and context = context_item list

exception Undecided (* raised if termination depth exceeded  *)

val eq : ?src_fields : Field_sources.t -> typ -> typ -> bool
val eq_kind : ?src_fields : Field_sources.t -> kind -> kind -> bool

val sub : ?src_fields : Field_sources.t -> typ -> typ -> bool
val sub_explained : ?src_fields : Field_sources.t -> context -> typ -> typ -> compatibility
val compatible : typ -> typ -> bool

exception PreEncountered
val lub : ?src_fields : Field_sources.t -> typ -> typ -> typ
val glb : ?src_fields : Field_sources.t -> typ -> typ -> typ


(* First-order substitution *)

val subst : typ ConEnv.t -> typ -> typ

val close : con list -> typ -> typ
val close_binds : con list -> bind list -> bind list

val open_ : typ list -> typ -> typ
val open_binds : bind list -> typ list


(* Environments *)

module Env : Env.S with type key = string


(* Scope bindings *)

val scope_var : var -> var
val default_scope_var : var
val scope_bound : typ
val scope_bind : bind

(* Signatures *)

(* like sub, but disallows promotion to  Any or narrower object types
   that signal data loss *)
val stable_sub : ?src_fields : Field_sources.t -> typ -> typ -> bool
val stable_sub_explained : ?src_fields : Field_sources.t -> context -> typ -> typ -> compatibility

type stab_sig =
  | Single of field list
  | PrePost of (bool * field) list * field list

val pre : stab_sig -> (bool * field) list
val post : stab_sig -> field list

val match_stab_sig : stab_sig -> stab_sig -> bool

val string_of_stab_sig : stab_sig -> string

val motoko_runtime_information_type : typ

(* Well-known labels *)

val cycles_lab : lab
val migration_lab : lab
val multi_migration_lab : lab
val timeout_lab : lab

(* Well-known fields *)

val motoko_async_helper_fld : field
val motoko_stable_var_info_fld : field
val motoko_gc_trigger_fld : field
val motoko_runtime_information_fld : field

val cycles_fld : field
val timeout_fld : field

val well_known_actor_fields : field list
val decode_msg_typ : field list -> typ

val canister_settings_typ : typ
val install_arg_typ : typ
val install_typ : typ list -> typ -> typ

(* Pretty printing *)

val string_of_prim : prim -> string
val string_of_obj_sort : obj_sort -> string
val string_of_func_sort : func_sort -> string

module type Pretty = sig
  val set_con_map : path ConEnv.t -> unit
  val clear_con_map : unit -> unit
  val pp_lab : Format.formatter -> lab -> unit
  val pp_typ : Format.formatter -> typ -> unit
  val pp_typ_expand : Format.formatter -> typ -> unit
  val pps_of_kind : kind ->
    string *
    (Format.formatter -> unit -> unit) *
      (Format.formatter -> unit -> unit)

  val string_of_con : con -> string
  val string_of_typ : typ -> string
  val string_of_kind : kind -> string
  val strings_of_kind : kind -> string * string * string
  val string_of_typ_expand : typ -> string
  val string_of_explanation : explanation -> string
  val is_redundant_explanation : typ -> typ -> explanation -> bool
end

module type PrettyConfig = sig
  val show_stamps : bool
  val show_scopes : bool
  val show_hash_suffix : bool
  val con_sep : string
  val par_sep : string
  val max_list : int option
end

module ShowStamps : PrettyConfig

module ElideStamps : PrettyConfig

module ParseableStamps : PrettyConfig

module ElideStampsAndHashes : PrettyConfig

module MakePretty(_ : PrettyConfig) : Pretty

include Pretty
