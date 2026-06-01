open Lazy

(* The type of outputters

 While decoding a type, we simultaneously build an outputter (IO action),
 that reads bytes from the stdin and dumps to stdout in a formatted way.
*)

type outputter = unit -> unit

(* noise reduction *)

let chatty = ref false

(* read nothing *)

let epsilon : outputter = ignore

(* reading at byte-level *)

let read_byte () : int =
  match In_channel.input_byte stdin with Some b -> b | None -> failwith "EOF"

let read_2byte () : int =
  let lsb = read_byte () in
  let msb = read_byte () in
  (msb lsl 8) + lsb

let read_4byte () : int =
  let lsb = read_2byte () in
  let msb = read_2byte () in
  (msb lsl 16) + lsb

let read_8byte () : Big_int.big_int =
  let lsb = read_4byte () in
  let msb = read_4byte () in
  Big_int.(
    add_int_big_int lsb (mult_int_big_int 4294967296 (big_int_of_int msb)))

let read_signed_byte () : bool * int =
  let b = read_byte () in
  if b > 127 then (true, b - 128) else (false, b)

let read_known char : unit =
  match read_byte () with
  | b when b = Char.code char -> ()
  | _ -> failwith "unexpected"

(* reading numbers *)

(* int-typed LEB/SLEB for byte counts, hashes, vector lengths and tag
   indices — values that always fit in OCaml's native int. *)

let read_leb128 () : int =
  let rec leb128 w : int =
    match read_signed_byte () with
    | true, n -> (w * n) + leb128 (w * 128)
    | _, n -> w * n
  in
  leb128 1

let read_sleb128 () : int =
  let rec sleb128 w : int =
    match read_signed_byte () with
    | true, n -> (w * n) + sleb128 (w * 128)
    | _, n -> w * if n > 63 then n - 128 else n
  in
  sleb128 1

(* Bignum LEB/SLEB for Candid `nat` / `int` values, which the spec
   does not bound.  Uses `num`/`Big_int` to align with the rest of
   moc (`mo_values/numerics.ml`); avoids pulling libgmp via zarith. *)

let read_leb128_bignum () : Big_int.big_int =
  let rec leb128 shift =
    match read_signed_byte () with
    | true, n ->
        let rest = leb128 (shift + 7) in
        Big_int.(add_big_int (shift_left_big_int (big_int_of_int n) shift) rest)
    | _, n -> Big_int.shift_left_big_int (Big_int.big_int_of_int n) shift
  in
  leb128 0

let read_sleb128_bignum () : Big_int.big_int =
  let rec sleb128 shift =
    match read_signed_byte () with
    | true, n ->
        let rest = sleb128 (shift + 7) in
        Big_int.(add_big_int (shift_left_big_int (big_int_of_int n) shift) rest)
    | _, n ->
        let signed = if n > 63 then n - 128 else n in
        Big_int.shift_left_big_int (Big_int.big_int_of_int signed) shift
  in
  sleb128 0

let read_int8 () : int =
  match read_signed_byte () with true, n -> n - 128 | _, n -> n

let read_int16 () : int =
  let lsb = read_byte () in
  let msb = read_int8 () in
  (msb lsl 8) lor lsb

let read_int32 () : int =
  let lsb = read_2byte () in
  let msb = read_int16 () in
  (msb lsl 16) lor lsb

let read_int64 () : Big_int.big_int =
  let lsb = read_4byte () in
  let msb = read_int32 () in
  Big_int.(
    add_int_big_int lsb (mult_int_big_int 4294967296 (big_int_of_int msb)))

let read_bool () : bool =
  match read_byte () with
  | 0 -> false
  | 1 -> true
  | _ -> failwith "invalid boolean"

(* IEEE-754 little-endian.  We unpack to native OCaml float (which is
   binary64); for float32 the bit pattern is widened via
   `Int32.float_of_bits` and stays representable exactly. *)

let read_float32 () : float =
  let lsw = read_2byte () in
  let msw = read_2byte () in
  Int32.float_of_bits
    (Int32.logor (Int32.shift_left (Int32.of_int msw) 16) (Int32.of_int lsw))

let read_float64 () : float =
  let lo = read_4byte () in
  let hi = read_4byte () in
  Int64.float_of_bits
    (Int64.logor (Int64.shift_left (Int64.of_int hi) 32) (Int64.of_int lo))

(* Principal — wire format: `i8(1) leb128(N) bytes(N)`.
   The 1-byte prefix distinguishes transparent (1) from opaque (0)
   references; only transparent ones can appear on the wire. Per the
   IC interface spec, principal blobs are bounded to 29 bytes. *)

let principal_max_len = 29

let read_principal () : bytes =
  match read_byte () with
  | 1 ->
      let n = read_leb128 () in
      if n > principal_max_len then
        failwith
          (Printf.sprintf "principal too long: %d bytes (spec maximum is %d)" n
             principal_max_len);
      let buf = Bytes.create n in
      really_input stdin buf 0 n;
      buf
  | 0 -> failwith "opaque principal references cannot appear on the wire"
  | _ -> failwith "invalid principal tag"

(* Principal blobs render through `Ic.Url.encode_principal`, which is
   the same CRC32 + base32 + dash-grouping encoder moc itself uses in
   `mo_values/value.ml` and `mo_values/show.ml`. *)

let principal_text (blob : bytes) : string =
  Ic.Url.encode_principal (Bytes.to_string blob)

(* Shared function reference — wire format:
   `i8(1) <principal> leb128(M) bytes(M)`. The outer `i8(1)` is the
   func transparent-reference tag; the principal carries its own
   inner `i8(1)` per `M(id : principal) = i8(1) M(id : bytes)`. *)

let read_func () : bytes * string =
  (match read_byte () with
  | 1 -> ()
  | 0 -> failwith "opaque function references cannot appear on the wire"
  | _ -> failwith "invalid function reference tag");
  let principal = read_principal () in
  let m = read_leb128 () in
  let method_name = Bytes.create m in
  really_input stdin method_name 0 m;
  (principal, Bytes.unsafe_to_string method_name)

(* Magic *)

let read_magic () : unit =
  read_known 'D';
  read_known 'I';
  read_known 'D';
  read_known 'L'

(* Repetition *)

let read_star_heralding a
    (heralder : int -> outputter * ('a -> int -> outputter -> outputter))
    (t : outputter) : unit =
  let rep = read_leb128 () in
  let herald_vector, herald_member = heralder rep in
  herald_vector ();
  for i = 0 to rep - 1 do
    herald_member a i t ()
  done

let read_t_star (t : unit -> 'a) : 'a array =
  let rep = read_leb128 () in
  Array.init rep (fun _ -> t ())

(* Annotations *)

type ann = Pure | Oneway

let read_annotation () : ann =
  match read_byte () with
  | 1 -> Pure
  | 2 -> Oneway
  | _ -> failwith "invalid annotation"

type typ =
  | Null
  | Bool
  | Nat
  | NatN of int
  | Int
  | IntN of int
  | Float32
  | Float64
  | Text
  | Reserved
  | Empty
  | Principal
  | Opt of typ Lazy.t
  | Vec of typ Lazy.t
  | Record of fields
  | Variant of alts
  | Function of typ Lazy.t array * typ Lazy.t array * ann array
  | Service of (string * typ Lazy.t) array
  | Future of int * Buffer.t

and alts = (int * typ Lazy.t) array
and fields = (int * typ Lazy.t) array

(* type index/ground type (negative) *)

let read_type_index () =
  let ty = read_sleb128 () in
  (* allow primitive codes (-1..-17), principal (-24), and positive
     indices into the type table; constructed types (-18..-23) cannot
     be referenced inline. *)
  assert (ty > -18 || ty = -24);
  ty

let read_assoc () =
  let hash = read_leb128 () in
  let tynum = read_type_index () in
  if !chatty then Printf.printf "hash: %d, tynum: %d\n" hash tynum;
  (hash, tynum)

type dump = {
  output_nat : Big_int.big_int -> unit;
  output_int : Big_int.big_int -> unit;
  output_bool : bool -> unit;
  output_nil : outputter;
  output_byte : int -> unit;
  output_2byte : int -> unit;
  output_4byte : int -> unit;
  output_8byte : Big_int.big_int -> unit;
  output_int8 : int -> unit;
  output_int16 : int -> unit;
  output_int32 : int -> unit;
  output_int64 : Big_int.big_int -> unit;
  output_float32 : float -> unit;
  output_float64 : float -> unit;
  output_text : int -> in_channel -> out_channel -> unit;
  output_principal : bytes -> unit;
  output_func : bytes -> string -> unit;
  output_service : bytes -> unit;
  output_some : outputter -> unit;
  output_arguments : int -> outputter * (unit -> int -> outputter -> outputter);
  output_vector : int -> outputter * (unit -> int -> outputter -> outputter);
  output_record : int -> outputter * (fields -> int -> outputter -> outputter);
  output_variant : int -> outputter * (alts -> int -> outputter -> outputter);
}

let prose : dump =
  let indent_amount = 4 in
  let indentation = ref 0 in
  let continue_line = ref false in
  let indent () = indentation := !indentation + indent_amount in
  let outdent () = indentation := !indentation - indent_amount in
  let ind i = if i = 0 then indent () in
  let outd max i = if i + 1 = max then outdent () in
  let bracket max g p i f () =
    ind i;
    g p i f;
    outd max i
  in
  let fill () =
    if !continue_line then (
      continue_line := false;
      "")
    else String.make !indentation ' '
  in
  let output_string what (s : string) =
    Printf.printf "%s%s: %s\n" (fill ()) what s
  in
  let output_decimal what (i : int) =
    Printf.printf "%s%s: %d\n" (fill ()) what i
  in
  let output_big_decimal what (i : Big_int.big_int) =
    Printf.printf "%s%s: %s\n" (fill ()) what (Big_int.string_of_big_int i)
  in
  let output_nat nat = output_big_decimal "output_nat" nat in
  let output_int n = output_big_decimal "output_int" n in
  let output_bool b =
    output_string "output_bool" (if b then "true" else "false")
  in
  let output_nil () = Printf.printf "%snull (0 bytes)\n" (fill ()) in
  let output_some consumer =
    Printf.printf "%sSome: value follows on the next line\n" (fill ());
    consumer ()
  in
  let output_byte b = output_decimal "output_byte" b in
  let output_2byte b = output_decimal "output_2byte" b in
  let output_4byte b = output_decimal "output_4byte" b in
  let output_8byte b = output_big_decimal "output_8byte" b in
  let output_int8 i = output_decimal "output_int8" i in
  let output_int16 i = output_decimal "output_int16" i in
  let output_int32 i = output_decimal "output_int32" i in
  let output_int64 i = output_big_decimal "output_int64" i in
  let output_float32 f = Printf.printf "%soutput_float32: %.9g\n" (fill ()) f in
  let output_float64 f =
    Printf.printf "%soutput_float64: %.17g\n" (fill ()) f
  in
  let output_text bytes from tostream =
    let buf = Buffer.create 0 in
    Buffer.add_channel buf from bytes;
    Printf.printf "%sText: %d bytes follow on next line\n" (fill ()) bytes;
    Printf.printf "%s---->" (fill ());
    Buffer.output_buffer tostream buf;
    print_string "\n"
  in
  let output_principal blob =
    Printf.printf "%soutput_principal: %s\n" (fill ()) (principal_text blob)
  in
  let output_func blob meth =
    Printf.printf "%soutput_func: %s . %s\n" (fill ()) (principal_text blob)
      meth
  in
  let output_service blob =
    Printf.printf "%soutput_service: %s\n" (fill ()) (principal_text blob)
  in
  let output_arguments args =
    let herald_arguments = function
      | () when args = 0 -> Printf.printf "%sNo arguments...\n" (fill ())
      | _ when args = 1 -> Printf.printf "%s1 argument follows\n" (fill ())
      | _ -> Printf.printf "%s%d arguments follow\n" (fill ()) args
    in
    let herald_member () i f =
      Printf.printf "%sArgument #%d%s: " (fill ()) i
        (if i + 1 = args then " (last)" else "");
      continue_line := true;
      f ()
    in
    (herald_arguments, bracket args herald_member)
  in
  let output_vector members =
    let herald_vector () =
      if members = 0 then Printf.printf "%sEmpty Vector\n" (fill ())
      else Printf.printf "%sVector with %d members follows\n" (fill ()) members
    in
    let herald_member () i f =
      Printf.printf "%sVector member %d%s: " (fill ()) i
        (if i + 1 = members then " (last)" else "");
      continue_line := true;
      f ()
    in
    (herald_vector, bracket members herald_member)
  in
  let output_record members =
    let herald_record () =
      if members = 0 then Printf.printf "%sEmpty Record\n" (fill ())
      else Printf.printf "%sRecord with %d members follows\n" (fill ()) members
    in
    let herald_member fields i f =
      Printf.printf "%sRecord member %d%s: " (fill ())
        (fst (Array.get fields i))
        (if i + 1 = members then " (last)" else "");
      continue_line := true;
      f ()
    in
    (herald_record, bracket members herald_member)
  in
  let output_variant members =
    let herald_variant () =
      assert (members <> 0);
      Printf.printf "%sVariant with %d members follows\n" (fill ()) members
    in
    let herald_member alts i f () =
      indent ();
      Printf.printf "%sVariant member %d: " (fill ()) (fst (Array.get alts i));
      continue_line := true;
      f ();
      outdent ()
    in
    (herald_variant, herald_member)
  in
  {
    output_nat;
    output_int;
    output_bool;
    output_nil;
    output_byte;
    output_2byte;
    output_4byte;
    output_8byte;
    output_int8;
    output_int16;
    output_int32;
    output_int64;
    output_float32;
    output_float64;
    output_text;
    output_principal;
    output_func;
    output_service;
    output_some;
    output_arguments;
    output_vector;
    output_record;
    output_variant;
  }

let idl : dump =
  let output_string (s : string) = print_string s in
  let chat_string s = if !chatty then output_string s in
  let output_string_space (s : string) =
    output_string s;
    output_string " "
  in
  let output_decimal (i : int) = Printf.printf "%d" i in
  let output_big_decimal (i : Big_int.big_int) =
    output_string (Big_int.string_of_big_int i)
  in
  let casted ty f v =
    match ty with
    | IntN n ->
        f v;
        Printf.printf " : int%d" n
    | NatN n ->
        f v;
        Printf.printf " : nat%d" n
    | _ -> assert false
  in
  let output_bool b = output_string (if b then "true" else "false") in
  let output_nil () = output_string "null" in
  let output_some consumer =
    output_string_space "opt";
    consumer ()
  in
  let output_byte = casted (NatN 8) output_decimal in
  let output_2byte = casted (NatN 16) output_decimal in
  let output_4byte = casted (NatN 32) output_decimal in
  let output_8byte (v : Big_int.big_int) =
    casted (NatN 64) output_big_decimal v
  in
  let output_nat = output_big_decimal in
  let output_int = output_big_decimal in
  let output_int8 = casted (IntN 8) output_decimal in
  let output_int16 = casted (IntN 16) output_decimal in
  let output_int32 = casted (IntN 32) output_decimal in
  let output_int64 (v : Big_int.big_int) =
    casted (IntN 64) output_big_decimal v
  in
  let output_float32 f = Printf.printf "%.9g : float32" f in
  let output_float64 f = Printf.printf "%.17g : float64" f in
  let output_text n froms tos =
    output_string "\"";
    let buf = Buffer.create 0 in
    Buffer.add_channel buf froms n;
    Buffer.output_buffer tos buf;
    output_string "\""
  in
  let output_principal blob =
    output_string "principal \"";
    output_string (principal_text blob);
    output_string "\""
  in
  let output_func blob meth =
    output_string "func \"";
    output_string (principal_text blob);
    output_string "\".\"";
    output_string meth;
    output_string "\""
  in
  let output_service blob =
    output_string "service \"";
    output_string (principal_text blob);
    output_string "\""
  in
  let output_arguments args =
    let last i = i + 1 = args in
    let herald_arguments = function
      | () when args = 0 ->
          chat_string "// No arguments...\n";
          output_string "()"
      | _ when args = 1 -> chat_string "// 1 argument follows\n"
      | _ -> if !chatty then Printf.printf "// %d arguments follow\n" args
    in
    let herald_member () i f () =
      if !chatty then
        Printf.printf "// Argument #%d%s:\n" i
          (if last i then " (last)" else "");
      output_string (if i = 0 then "( " else ", ");
      f ();
      output_string (if last i then "\n)\n" else "\n")
    in
    (herald_arguments, herald_member)
  in
  let start i = if i = 0 then output_string_space "{" in
  let stop max i = if i + 1 = max then output_string " }" in
  let bracket max g p i f () =
    start i;
    g p i f;
    stop max i
  in
  let output_vector members =
    let herald_vector () =
      if members = 0 then output_string_space "vec { }"
      else output_string_space "vec"
    in
    let herald_member () i f =
      f ();
      output_string_space ";"
    in
    (herald_vector, bracket members herald_member)
  in
  let output_record members =
    let herald_record () =
      if members = 0 then output_string_space "record { }"
      else output_string_space "record"
    in
    let herald_member fields i f =
      Printf.printf "%d : " (fst (Array.get fields i));
      f ();
      output_string_space ";"
    in
    (herald_record, bracket members herald_member)
  in
  let output_variant members =
    let herald_variant () =
      assert (members <> 0);
      output_string_space "variant"
    in
    let herald_member alts i f () =
      start 0;
      Printf.printf "%d : " (fst (Array.get alts i));
      f ();
      stop 1 0
    in
    (herald_variant, herald_member)
  in
  {
    output_nat;
    output_int;
    output_bool;
    output_nil;
    output_byte;
    output_2byte;
    output_4byte;
    output_8byte;
    output_int8;
    output_int16;
    output_int32;
    output_int64;
    output_float32;
    output_float64;
    output_text;
    output_principal;
    output_func;
    output_service;
    output_some;
    output_arguments;
    output_vector;
    output_record;
    output_variant;
  }

let json : dump =
  let output_string (s : string) = print_string s in
  let output_string_space (s : string) = Printf.printf "%s " s in
  let output_decimal (i : int) = Printf.printf "%d" i in
  let output_big_decimal (i : Big_int.big_int) =
    output_string (Big_int.string_of_big_int i)
  in
  let output_bool b = output_string (if b then "true" else "false") in
  let output_nil () = output_string "null" in
  let output_some consumer =
    output_string "[";
    consumer ();
    output_string "]"
  in
  let output_byte = output_decimal in
  let output_2byte = output_decimal in
  let output_4byte = output_decimal in
  let output_8byte (v : Big_int.big_int) = output_big_decimal v in
  let output_nat = output_big_decimal in
  let output_int = output_big_decimal in
  let output_int8 = output_decimal in
  let output_int16 = output_decimal in
  let output_int32 = output_decimal in
  let output_int64 (v : Big_int.big_int) = output_big_decimal v in
  let output_float32 f = Printf.printf "%.9g" f in
  let output_float64 f = Printf.printf "%.17g" f in
  let output_text n froms tos =
    output_string "\"";
    let buf = Buffer.create 0 in
    Buffer.add_channel buf froms n;
    Buffer.output_buffer tos buf;
    output_string "\""
  in
  let output_principal blob =
    output_string "\"";
    output_string (principal_text blob);
    output_string "\""
  in
  let output_func blob meth =
    output_string "{\"principal\": \"";
    output_string (principal_text blob);
    output_string "\", \"method\": \"";
    output_string meth;
    output_string "\"}"
  in
  let output_service blob =
    output_string "\"";
    output_string (principal_text blob);
    output_string "\""
  in
  let output_arguments args =
    let herald_arguments = function
      | () when args = 0 -> output_string "# No arguments...\n"
      | _ when args = 1 -> output_string "# 1 argument follows"
      | _ -> Printf.printf "# %d arguments follow" args
    in
    let herald_member () i f () =
      Printf.printf "\n# Argument #%d%s:\n" i
        (if i + 1 = args then " (last)" else "");
      f ();
      if i + 1 = args then print_newline ()
    in
    (herald_arguments, herald_member)
  in
  let start punct i = if i = 0 then output_string (String.make 1 punct) in
  let stop punct max i =
    if i + 1 = max then output_string (String.make 1 punct)
  in
  let bracket punct max g p i f () =
    start punct.[0] i;
    g p i f;
    stop punct.[1] max i
  in
  let output_vector members =
    let punct = "[]" in
    let herald_vector () = if members = 0 then output_string_space punct in
    let herald_member () i f =
      if i > 0 then output_string_space ",";
      f ()
    in
    (herald_vector, bracket punct members herald_member)
  in
  let output_record members =
    let punct = "{}" in
    let herald_record () = if members = 0 then output_string_space punct in
    let herald_member fields i f =
      if i > 0 then output_string_space ",";
      Printf.printf "\"_%d_\": " (fst (Array.get fields i));
      f ()
    in
    (herald_record, bracket punct members herald_member)
  in
  let output_variant members =
    let herald_variant () = assert (members <> 0) in
    let herald_member alts i f () =
      start '{' 0;
      Printf.printf "_%d_ : " (fst (Array.get alts i));
      f ();
      stop '}' 1 0
    in
    (herald_variant, herald_member)
  in
  {
    output_nat;
    output_int;
    output_bool;
    output_nil;
    output_byte;
    output_2byte;
    output_4byte;
    output_8byte;
    output_int8;
    output_int16;
    output_int32;
    output_int64;
    output_float32;
    output_float64;
    output_text;
    output_principal;
    output_func;
    output_service;
    output_some;
    output_arguments;
    output_vector;
    output_record;
    output_variant;
  }

let make_outputter (d : dump) : unit =
  let {
    output_nat;
    output_int;
    output_bool;
    output_nil;
    output_byte;
    output_2byte;
    output_4byte;
    output_8byte;
    output_int8;
    output_int16;
    output_int32;
    output_int64;
    output_float32;
    output_float64;
    output_text;
    output_principal;
    output_func;
    output_service;
    output_some;
    output_arguments;
    output_vector;
    output_record;
    output_variant;
  } =
    d
  in
  let decode_primitive_type : int -> typ * outputter = function
    | -1 -> (Null, output_nil)
    | -2 -> (Bool, fun () -> output_bool (read_bool ()))
    | -3 -> (Nat, fun () -> output_nat (read_leb128_bignum ()))
    | -4 -> (Int, fun () -> output_int (read_sleb128_bignum ()))
    | -5 -> (NatN 8, fun () -> output_byte (read_byte ()))
    | -6 -> (NatN 16, fun () -> output_2byte (read_2byte ()))
    | -7 -> (NatN 32, fun () -> output_4byte (read_4byte ()))
    | -8 -> (NatN 64, fun () -> output_8byte (read_8byte ()))
    | -9 -> (IntN 8, fun () -> output_int8 (read_int8 ()))
    | -10 -> (IntN 16, fun () -> output_int16 (read_int16 ()))
    | -11 -> (IntN 32, fun () -> output_int32 (read_int32 ()))
    | -12 -> (IntN 64, fun () -> output_int64 (read_int64 ()))
    | -13 -> (Float32, fun () -> output_float32 (read_float32 ()))
    | -14 -> (Float64, fun () -> output_float64 (read_float64 ()))
    | -15 ->
        ( Text,
          fun () ->
            let len = read_leb128 () in
            output_text len stdin stdout )
    | -16 -> (Reserved, ignore)
    | -17 -> (Empty, ignore)
    | -24 -> (Principal, fun () -> output_principal (read_principal ()))
    | _ -> failwith "unrecognised primitive type"
  in
  let read_type lookup : (typ * outputter) Lazy.t =
    let lprim_or_lookup = function
      | -24 -> lazy (decode_primitive_type (-24))
      | p when p < -17 -> assert false
      | p when p < 0 -> lazy (decode_primitive_type p)
      | i -> lookup i
    in
    let prim_or_lookup ty = force (lprim_or_lookup ty) in
    let lfst p =
      lazy
        (let (lazy (f, _)) = p in
         f)
    in
    let lsnd p =
      lazy
        (let (lazy (_, s)) = p in
         s)
    in
    match read_sleb128 () with
    | p when (p < 0 && p > -18) || p = -24 -> from_val (decode_primitive_type p)
    | -18 ->
        let reader consumer () =
          match read_byte () with
          | 0 -> output_nil ()
          | 1 -> output_some (force consumer)
          | _ -> failwith "invalid optional"
        in
        let i = read_type_index () in
        lazy
          (let p = lprim_or_lookup i in
           (Opt (lfst p), reader (lsnd p)))
    | -19 ->
        let i = read_type_index () in
        lazy
          (let p = lprim_or_lookup i in
           ( Vec (lfst p),
             fun () -> read_star_heralding () output_vector (force (lsnd p)) ))
    | -20 ->
        let assocs = read_t_star read_assoc in
        lazy
          (let herald_record, herald_member =
             output_record (Array.length assocs)
           in
           let members =
             Array.map
               (fun (i, tynum) -> (i, lfst (lprim_or_lookup tynum)))
               assocs
           in
           let consumers =
             Array.mapi (herald_member members)
               (Array.map
                  (fun (_, tynum) () -> snd (prim_or_lookup tynum) ())
                  assocs)
           in
           ( Record members,
             fun () ->
               herald_record ();
               Array.iter (fun f -> f ()) consumers ))
    | -21 ->
        let assocs = read_t_star read_assoc in
        lazy
          (let herald_variant, herald_member =
             output_variant (Array.length assocs)
           in
           let alts =
             Array.map
               (fun (i, tynum) -> (i, lfst (lprim_or_lookup tynum)))
               assocs
           in
           let consumers =
             Array.map
               (fun (_, tynum) () -> snd (prim_or_lookup tynum) ())
               assocs
           in
           ( Variant alts,
             fun () ->
               herald_variant ();
               let i = read_leb128 () in
               herald_member alts i (Array.get consumers i) () ))
    | -22 ->
        let types1 = read_t_star read_type_index in
        let types2 = read_t_star read_type_index in
        let anns = read_t_star read_annotation in
        lazy
          (let args =
             Array.map (fun tynum -> lfst (lprim_or_lookup tynum)) types1
           in
           let rslts =
             Array.map (fun tynum -> lfst (lprim_or_lookup tynum)) types2
           in
           ( Function (args, rslts, anns),
             fun () ->
               let p, m = read_func () in
               output_func p m ))
    (*
T(service {<methtype>*}) = sleb128(-23) T*(<methtype>* )
*)
    | -23 ->
        let read_methtype () =
          let n = read_leb128 () in
          let name = Bytes.create n in
          really_input stdin name 0 n;
          let tynum = read_type_index () in
          (Bytes.unsafe_to_string name, tynum)
        in
        let methods = read_t_star read_methtype in
        lazy
          (let method_types =
             Array.map (fun (n, t) -> (n, lfst (lprim_or_lookup t))) methods
           in
           (Service method_types, fun () -> output_service (read_principal ())))
    | t ->
        (* future type *)
        let bytes = read_leb128 () in
        let buf = Buffer.create 0 in
        Buffer.add_channel buf stdin bytes;
        let ingest () =
          let bytes = read_leb128 () in
          let refs = read_leb128 () in
          let buf = Buffer.create 0 in
          assert (refs = 0);
          Buffer.add_channel buf stdin bytes
        in
        lazy (Future (t, buf), ingest)
  in
  let read_type_table (t : unit -> (typ * outputter) Lazy.t) :
      (typ * outputter) Lazy.t array =
    let rep = read_leb128 () in
    Array.init rep (fun i ->
        if !chatty then Printf.printf "read_type_table: %d\n" i;
        t ())
  in
  let chat_string = if !chatty then print_string else ignore in
  chat_string "\nDESER, to your service!\n";
  read_magic ();
  chat_string "\n========================== Type section\n";
  let tab =
    let rec tab = lazy (read_type_table (fun () -> read_type lookup))
    and lookup =
     fun indx ->
      (*Printf.printf "{indx: %d}" indx; *) Array.get (force tab) indx
    in
    Array.map force (force tab)
  in
  chat_string "\n========================== Value section\n";
  let argtys = read_t_star read_type_index in
  let herald_arguments, herald_member =
    output_arguments (Array.length argtys)
  in
  herald_arguments ();
  let typ_ingester = function
    | prim when prim < 0 -> decode_primitive_type prim
    | index -> Array.get tab index
  in
  let consumers =
    Array.map
      (fun tynum ->
        let ty, m = typ_ingester tynum in
        m)
      argtys
  in
  Array.iteri (fun i f -> herald_member () i f ()) consumers;
  chat_string "\n-------- DESER DONE\n"

(* CLI *)

let name = "deser"
let banner = "Candid toolkit " ^ Source_id.banner
let usage = "Usage: " ^ name ^ " [option] [file ...]"

type format = Idl | Prose | Json

let output_format = ref Idl

let set_format f () =
  if !output_format <> Idl then begin
    Printf.eprintf "deser: multiple output formats specified";
    exit 1
  end;
  output_format := f

let argspec =
  Arg.align
    [
      ("--prose", Arg.Unit (set_format Prose), " output indented prose");
      ("--json", Arg.Unit (set_format Json), " output JSON values");
      ("--idl", Arg.Unit (set_format Idl), " output IDL values (default)");
      ("--verbose", Arg.Unit (fun () -> chatty := true), " amend commentary");
      ( "--version",
        Arg.Unit
          (fun () ->
            Printf.printf "%s\n" banner;
            exit 0),
        " show version" );
    ]

let add_arg source = () (* args := !args @ [source] *)

(* run it *)

let () =
  Arg.parse argspec add_arg usage;
  let dump =
    match !output_format with Prose -> prose | Idl -> idl | Json -> json
  in
  make_outputter dump;
  match In_channel.input_byte stdin with
  | Some _ -> failwith "surplus bytes in input"
  | None -> ()

(* TODOs:
  - escaping in text
  - heralding/outputting of type table
 *)
