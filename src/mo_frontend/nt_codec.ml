(* The codec of a native table: two Blobs that describe to the runtime the rows
   of a table with key type K and row type R.

   K is a scalar. R is a scalar, or a record of immutable fields where each
   field is a scalar or an option of one. The scalars are Nat, Int, Nat8 to
   Nat64, Int8 to Int64, Float, Bool, Char, Text, Blob and Principal.

   `layout`, all numbers little-endian:
     4 bytes  magic "NTCD"
     u32      version, 1
     u64      codec hash: the native-table crate's nt_hash64 of all bytes after it
     u32      column count
     then per column, the key first, then the fields of R in label-hash order:
       1 or 2 bytes  the crate's type code of the scalar, after the option
                     code 0x20 when the field is optional
       u32           the field's label hash; 0 for the key and for a scalar R
       u16           the length of the field's name, then its bytes; the key
                     and a scalar R have no name

   `labels`: the label hashes of R's fields as u64 in ascending order. These
   are the bytes of the hash list that compiled code attaches to a record of
   type R. Empty when R is a scalar.

   Other types, and types with type parameters, are rejected with a message
   that names the key, the row or the field.
*)

open Mo_types

module T = Type

let magic = "NTCD"
let version = 1l
let option_code = 0x20

(* The native-table crate's type codes *)
let scalar_code : T.prim -> int option = function
  | T.Bool -> Some 0x01
  | T.Nat8 -> Some 0x02
  | T.Nat16 -> Some 0x03
  | T.Nat32 -> Some 0x04
  | T.Nat64 -> Some 0x05
  | T.Int8 -> Some 0x06
  | T.Int16 -> Some 0x07
  | T.Int32 -> Some 0x08
  | T.Int64 -> Some 0x09
  | T.Nat -> Some 0x0A
  | T.Int -> Some 0x0B
  | T.Float -> Some 0x0D
  | T.Char -> Some 0x0E
  | T.Text -> Some 0x0F
  | T.Blob -> Some 0x10
  | T.Principal -> Some 0x11
  | _ -> None

let scalar t =
  match T.normalize t with
  | T.Prim p -> Option.map (fun c -> [c]) (scalar_code p)
  | _ -> None

let field_type t =
  match T.normalize t with
  | T.Opt t1 -> Option.map (fun cs -> option_code :: cs) (scalar t1)
  | _ -> scalar t

let is_param t =
  match T.normalize t with
  | T.Con (c, _) -> (match Cons.kind c with T.Abs _ -> true | T.Def _ -> false)
  | _ -> false

(* The message for [what] (the key, the row or a field) of type [t] that breaks [rule] *)
let reject what t rule =
  let param = match T.normalize t with T.Opt t1 -> is_param t1 | _ -> is_param t in
  Error (Printf.sprintf "native table %s has type %s, but %s" what (T.string_of_typ t)
    (if param then "its type must not depend on a type parameter" else rule))

let key_rule = "a key must be one of Nat, Int, Nat8 to Nat64, Int8 to Int64, Float, Bool, Char, Text, Blob or Principal"
let row_rule = "a row must be a scalar or a record"
let field_rule = "a field must be a scalar or an option of a scalar"

(* A column: its type codes, label hash and name *)
type column = { codes : int list; hash : int32; name : string }

let unnamed codes = { codes; hash = 0l; name = "" }

(* The columns of R, and whether R is a record *)
let row_columns r =
  let open Lib.Result.Syntax in
  match T.normalize r with
  | T.Obj (T.Object, fs, []) ->
    let field (f : T.field) =
      let what = Printf.sprintf "field `%s`" f.T.lab in
      if T.is_mut f.T.typ then reject what f.T.typ "a field must be immutable" else
      match field_type f.T.typ with
      | None -> reject what f.T.typ field_rule
      | Some _ when String.length f.T.lab > 0xFFFF ->
        Error (Printf.sprintf "native table %s has a name longer than 65535 bytes" what)
      | Some codes -> Ok { codes; hash = Hash.hash f.T.lab; name = f.T.lab } in
    let rec fields = function
      | [] -> Ok []
      | f :: fs ->
        let* c = field f in
        let* cs = fields fs in
        Ok (c :: cs) in
    let* cs = fields fs in
    Ok (List.sort (fun c1 c2 -> Int32.compare c1.hash c2.hash) cs, true)
  | _ ->
    match scalar r with
    | None -> reject "row" r row_rule
    | Some codes -> Ok ([unnamed codes], false)

(* splitmix64's finalizer *)
let mix z =
  let open Int64 in
  let z = logxor z (shift_right_logical z 30) in
  let z = mul z 0xBF58476D1CE4E5B9L in
  let z = logxor z (shift_right_logical z 27) in
  let z = mul z 0x94D049BB133111EBL in
  logxor z (shift_right_logical z 31)

(* The native-table crate's nt_hash64: one mix per 8-byte little-endian word,
   the last one zero-padded *)
let hash64 s =
  let n = String.length s in
  let padded = s ^ String.make ((8 - n mod 8) mod 8) '\000' in
  let h = ref (Int64.logxor 0x9E3779B97F4A7C15L (Int64.of_int n)) in
  for i = 0 to String.length padded / 8 - 1 do
    h := mix (Int64.logxor !h (String.get_int64_le padded (8 * i)))
  done;
  !h

(* The layout and labels Blobs for key type [k] and row type [r], or why the
   types are not accepted *)
let blobs k r : (string * string, string) result =
  let open Lib.Result.Syntax in
  let* key = match scalar k with
    | None -> reject "key" k key_rule
    | Some codes -> Ok codes in
  let* (row, is_record) = row_columns r in
  let columns = unnamed key :: row in
  let body = Buffer.create 64 in
  Buffer.add_int32_le body (Int32.of_int (List.length columns));
  List.iter (fun c ->
    List.iter (Buffer.add_uint8 body) c.codes;
    Buffer.add_int32_le body c.hash;
    Buffer.add_uint16_le body (String.length c.name);
    Buffer.add_string body c.name
  ) columns;
  let body = Buffer.contents body in
  let layout = Buffer.create (16 + String.length body) in
  Buffer.add_string layout magic;
  Buffer.add_int32_le layout version;
  Buffer.add_int64_le layout (hash64 body);
  Buffer.add_string layout body;
  let labels = Buffer.create 64 in
  if is_record then
    List.iter (fun c -> Buffer.add_int64_le labels (Int64.of_int32 c.hash)) row;
  Ok (Buffer.contents layout, Buffer.contents labels)
