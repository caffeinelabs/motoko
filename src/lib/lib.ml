module StdList = List

module Format =
struct
  let with_str_formatter f x =
    let b = Buffer.create 16 in
    let ppf = Format.formatter_of_buffer b in
    Format.pp_set_geometry ppf ~max_indent:2 ~margin:(1000000010-1); (* hack to output all on one line *)
    Format.fprintf ppf "@[%a@]" f x;
    Format.pp_print_flush ppf ();
    Buffer.contents b

  let display pp ppf x =
    Format.fprintf ppf "@\n@[<v 2>  %a@]" pp x

end

module Fun =
struct
  let curry f x y = f (x, y)
  let uncurry f (x, y) = f x y

  let rec repeat n f x =
    if n = 0 then () else (f x; repeat (n - 1) f x)
end

module Int =
struct
  let log2 n =
    if n <= 0 then failwith "log2";
    let rec loop acc n = if n = 1 then acc else loop (acc + 1) (n lsr 1) in
    loop 0 n

  let is_power_of_two n =
    if n < 0 then failwith "is_power_of_two";
    n <> 0 && n land (n - 1) = 0
end

module Uint32 =
struct
  type t = int32
  let of_string str = Int32.of_string ("0u" ^ str)
  let of_string_opt str = Int32.of_string_opt ("0u" ^ str)
  let to_string n = Printf.sprintf "%lu" n
  let add = Int32.add
  let sub = Int32.sub
  let mul = Int32.mul
  let succ = Int32.succ
  let zero = Int32.zero
  let one = Int32.one
  let of_int = Int32.of_int
  let to_int = Int32.to_int
  let logand = Int32.logand
  let logor = Int32.logor
  let shift_right_logical = Int32.shift_right_logical
  let of_int32 x = x
  let to_int32 x = x
  let compare i1 i2 =
    if i1 < 0l && i2 >= 0l then 1
    else if i1 >= 0l && i2 < 0l then -1
    else Int32.compare i1 i2
end

module CRC =
struct
  let crc8 (bs : string) : int =
    let inner _ = function
      | crc when crc land 0x80 <> 0 -> (crc lsl 1) lxor 0x7
      | crc -> crc lsl 1 in
    let outer crc b =
      List.fold_right inner [0;1;2;3;4;5;6;7] (Char.code b lxor crc) land 0xFF in
    Seq.fold_left outer 0 (String.to_seq bs)

  let crc32 (bs : string) : int32 =
    Optint.(to_int32 (Checkseum.Crc32.digest_string bs 0 (String.length bs) zero))
end

module Hex =
struct
  let hexdigit = let open Char in function
    | c when c >= '0' && c <= '9' -> code c - code '0'
    | c when c >= 'A' && c <= 'F' -> code c - code 'A' + 10
    | c when c >= 'a' && c <= 'f' -> code c - code 'a' + 10
    | _ -> assert false

  let bytes_of_hex hex : string =
    let open String in
    let extract i _ =
      Char.chr (hexdigit (get hex (i * 2)) lsl 4 lor hexdigit (get hex (i * 2 + 1))) in
    Bytes.to_string (Bytes.mapi extract (Bytes.create (length hex / 2)))

  let int_of_hex_byte hex : int =
    assert (String.length hex = 2);
    String.(hexdigit (get hex 0) lsl 4 lor hexdigit (get hex 1))

  let hex_of_nibble =
    let open Char in
    function
    | c when 0 <= c && c <= 9 -> chr (code '0' + c)
    | c when 10 <= c && c <= 15 -> chr (code 'A' + (c - 10))
    | _ -> assert false

  let hex_of_byte i : string =
    String.init 2 (function
      | 0 -> hex_of_nibble (i / 16)
      | 1 -> hex_of_nibble (i mod 16)
      | _ -> assert false)

  let hex_of_char c = hex_of_byte (Char.code c)

  let hex_of_bytes bytes : string =
    let open Stdlib.String in
    of_seq (Stdlib.Seq.flat_map (fun c -> to_seq (hex_of_char c)) (to_seq bytes))
end

module Base32 =
struct
  let decode input =
    let len = String.length input in
    let buf = Buffer.create (len / 2) in
    let rec evac = function
      | v, b when b >= 8 ->
        let b' = b - 8 in
        Buffer.add_uint8 buf (v lsr b');
        evac (v land (1 lsl b' - 1), b')
      | vb -> vb in
    let b32 a = function
      | v when v >= 'A' && v <= 'Z' -> a lsl 5 lor (Char.code v - 65)
      | v when v >= '2' && v <= '7' -> a lsl 5 lor (Char.code v - 24)
      | '=' -> a
      | _ -> raise (Invalid_argument "Char out of base32 alphabet") in
      let pump (v, b) c = evac (b32 v c, b + 5) in
    try
      ignore (Seq.fold_left pump (0, 0) (String.to_seq input));
      Ok (Buffer.contents buf)
    with Invalid_argument s -> Error s

  let encode input =
    let len = String.length input in
    let buf = Buffer.create (len * 2) in
    let b32 = function
      | v when v <= 25 -> 65 + v
      | v -> 24 + v in
    let rec evac = function
      | v, b when b >= 5 ->
        let b' = b - 5 in
        Buffer.add_uint8 buf (b32 (v lsr b'));
        evac (v land (1 lsl b' - 1), b')
      | vb -> vb
    in
    let pump (v, b) c = evac (v lsl 8 lor (Char.code c land 0xFF), b + 8) in
    let v, b = Seq.fold_left pump (0, 0) (String.to_seq input) in
    if b > 0 then ignore (evac (v lsl 4, b + 4));
    Buffer.contents buf
end

module String =
struct

  let implode cs =
    let buf = Buffer.create 80 in
    List.iter (Buffer.add_char buf) cs;
    Buffer.contents buf

  let implode_map f cs =
    let buf = Buffer.create 80 in
    List.iter (fun c -> Buffer.add_char buf (f c)) cs;
    Buffer.contents buf

  let explode s =
    let cs = ref [] in
    for i = String.length s - 1 downto 0 do cs := s.[i] :: !cs done;
    !cs

  let explode_map f s =
    let cs = ref [] in
    for i = String.length s - 1 downto 0 do cs := f s.[i] :: !cs done;
    !cs

  (** Stack.fold (fun x y -> y ^ c ^ x) "" (String.split s c) == s *)
  let split s c =
    let len = String.length s in
    let rec loop i =
      if i > len then [] else
      let j = try String.index_from s i c with Not_found -> len in
      String.sub s i (j - i) :: loop (j + 1)
    in loop 0

  let breakup s n =
    let rec loop i =
      let len = min n (String.length s - i) in
      if len = 0 then [] else String.sub s i len :: loop (i + len)
    in loop 0

  let rec find_from_opt f s i =
    if i = String.length s then
      None
    else if f s.[i] then
      Some i
    else
      find_from_opt f s (i + 1)

  let chop_prefix prefix s =
    let prefix_len = String.length prefix in
    let s_len = String.length s in
    if s_len < prefix_len then
      None
    else if String.sub s 0 prefix_len = prefix then
      Some (String.sub s prefix_len (s_len - prefix_len))
    else
      None

  let chop_suffix suffix s =
    let suffix_len = String.length suffix in
    let s_len = String.length s in
    if s_len < suffix_len then
      None
    else if String.sub s (s_len - suffix_len) suffix_len = suffix then
      Some (String.sub s 0 (s_len - suffix_len))
    else
      None

  let lightweight_escaped s =
    let buf = Buffer.create (String.length s) in
    for i = 0 to String.length s - 1 do
      match s.[i] with
      | '\"' | '\'' | '\\' as c ->
        Buffer.add_char buf '\\'; Buffer.add_char buf c
      | '\n' -> Buffer.add_string buf "\\n"
      | '\r' -> Buffer.add_string buf "\\r"
      | '\t' -> Buffer.add_string buf "\\t"
      | c -> Buffer.add_char buf c
    done;
    Buffer.contents buf

  (* Courtesy of Claude.ai *)
  let levenshtein_distance s t =
    let m = String.length s
    and n = String.length t in

    (* Ensure s is the shorter string for optimization *)
    let (s, t, m, n) = if m > n then (t, s, n, m) else (s, t, m, n) in

    (* Initialize the previous row *)
    let previous_row = Array.init (m + 1) (fun i -> i) in

    (* Compute the distance *)
    for i = 1 to n do
      let current_row = Array.make (m + 1) 0 in
      current_row.(0) <- i;

      for j = 1 to m do
        let cost = if s.[j-1] = t.[i-1] then 0 else 1 in
        current_row.(j) <- min
          (min
            (previous_row.(j) + 1)     (* Deletion *)
            (current_row.(j-1) + 1)    (* Insertion *)
          )
          (previous_row.(j-1) + cost)  (* Substitution *)
      done;

      (* Swap rows *)
      Array.blit current_row 0 previous_row 0 (m + 1)
    done;

    (* Return the distance *)
    previous_row.(m)

end

module Utf8 =
struct
  type t = int list
  exception Utf8 = Wasm.Utf8.Utf8

  let rec is_valid s = is_valid' (String.explode_map Char.code s)
  and is_valid' = function
    | [] -> true
    | b1::bs when b1 < 0x80 ->
      is_valid' bs
    | b1::bs when b1 < 0xc0 -> false
    | b1::b2::bs when b1 < 0xe0 ->
      (b2 land 0xc0 = 0x80) && is_valid' bs
    | b1::b2::b3::bs when b1 < 0xf0 ->
      (b2 land 0xc0 = 0x80) && (b3 land 0xc0 = 0x80) && is_valid' bs
    | b1::b2::b3::b4::bs when b1 < 0xf8 ->
      (b2 land 0xc0 = 0x80) && (b3 land 0xc0 = 0x80) && (b4 land 0xc0 = 0x80) && is_valid' bs
    | _ -> false

  let con b = if b land 0xc0 = 0x80 then b land 0x3f else raise Utf8
  let code min n =
    if n < min || (0xd800 <= n && n < 0xe000) || n >= 0x110000 then raise Utf8
    else n

  let rec decode s = decode' [] (String.explode_map Char.code s)
  and decode' acc = function
    | [] -> List.rev acc
    | b1::bs when b1 < 0x80 ->
      decode' (code 0x0 b1 :: acc) bs
    | b1::bs when b1 < 0xc0 -> raise Utf8
    | b1::b2::bs when b1 < 0xe0 ->
      decode' (code 0x80 ((b1 land 0x1f) lsl 6 + con b2) :: acc) bs
    | b1::b2::b3::bs when b1 < 0xf0 ->
      decode' (code 0x800 ((b1 land 0x0f) lsl 12 + con b2 lsl 6 + con b3) :: acc) bs
    | b1::b2::b3::b4::bs when b1 < 0xf8 ->
      decode' (code 0x10000 ((b1 land 0x07) lsl 18 + con b2 lsl 12 + con b3 lsl 6 + con b4) :: acc) bs
    | _ -> raise Utf8

  let con n = 0x80 lor (n land 0x3f)

  let rec encode ns = String.implode_map Char.chr (encode' [] ns)
  and encode' acc = function
    | [] -> List.rev acc
    | n::ns when n < 0 -> raise Utf8
    | n::ns when n < 0x80 ->
      encode' (n :: acc) ns
    | n::ns when n < 0x800 ->
      encode' (con n :: 0xc0 lor (n lsr 6) :: acc) ns
    | n::ns when n < 0x10000 ->
      encode' (con n :: con (n lsr 6) :: 0xe0 lor (n lsr 12) :: acc) ns
    | n::ns when n < 0x110000 ->
      encode' (con n :: con (n lsr 6) :: con (n lsr 12) :: 0xf0 lor (n lsr 18) :: acc) ns
    | _ -> raise Utf8
end

type ('a, 'b) these =
  | This of 'a
  | That of 'b
  | Both of 'a * 'b

module List =
struct
  let equal p xs ys =
    try List.for_all2 p xs ys with _ -> false

  let rec make n x = make' n x []
  and make' n x xs =
    if n = 0 then xs else make' (n - 1) x (x::xs)

  let rec table n f = table' n f []
  and table' n f xs =
    if n = 0 then xs else table' (n - 1) f (f (n - 1) :: xs)

  let group f l =
    let rec grouping acc = function
      | [] -> acc
      | hd::tl ->
         let l1,l2 = List.partition (f hd) tl in
         grouping ((hd::l1)::acc) l2
    in grouping [] l

  let rec take n xs = (* present in OCaml 5.3 *)
    match n, xs with
    | _ when n <= 0 -> []
    | n, x::xs' when n > 0 -> x :: take (n - 1) xs'
    | _ -> failwith "take"

  let rec drop n xs = (* present in OCaml 5.3 *)
    match n, xs with
    | 0, _ -> xs
    | n, _::xs' when n > 0 -> drop (n - 1) xs'
    | _ -> failwith "drop"

  let rec replicate e = function
    | 0 -> []
    | n -> e :: replicate e (n - 1)

  let split_at n xs =
    if n <= List.length xs
    then (take n xs, drop n xs)
    else (xs, [])

  let split3 l =
    let rec loop (xs, ys, zs) = function
      | [] -> (List.rev xs, List.rev ys, List.rev zs)
      | (x, y, z) :: t -> loop (x :: xs, y :: ys, z :: zs) t
    in loop ([], [], []) l

  let mapi2 f xs ys =
    let _, acc = List.fold_left2 (fun (i, acc) x y -> (1 + i, f i x y :: acc)) (0, []) xs ys in
    List.rev acc

  let hd_opt = function
    | x :: _ -> Some x
    | _ -> None

  let rec last = function
    | [x] -> x
    | _::xs -> last xs
    | [] -> failwith "last"

  let last_opt = function
    | [] -> None
    | xs -> Some (last xs)

  let rec split_last = function
    | [x] -> [], x
    | x::xs -> let ys, y = split_last xs in x::ys, y
    | [] -> failwith "split_last"

  let rec index_where p xs = index_where' p xs 0
  and index_where' p xs i =
    match xs with
    | [] -> None
    | x::xs' when p x -> Some i
    | x::xs' -> index_where' p xs' (i+1)

  let index_of x = index_where ((=) x)

  let rec compare f xs ys =
    match xs, ys with
    | [], [] -> 0
    | [], _ -> -1
    | _, [] -> +1
    | x::xs', y::ys' ->
      match f x y with
      | 0 -> compare f xs' ys'
      | n -> n

  let rec is_ordered f xs =
    match xs with
    | [] | [_] -> true
    | x1::x2::xs' ->
      match f x1 x2 with
      | -1 | 0 -> is_ordered f (x2::xs')
      | _ -> false

  let rec is_strictly_ordered f xs =
    match xs with
    | [] | [_] -> true
    | x1::x2::xs' ->
      match f x1 x2 with
      | -1 -> is_strictly_ordered f (x2::xs')
      | _ -> false

  let rec iter_pairs f = function
    | [] -> ()
    | x::ys -> List.iter (fun y -> f x y) ys; iter_pairs f ys

  let rec is_prefix equal prefix list =
    match prefix with
    | [] -> true
    | hd :: tl ->
      (match list with
       | [] -> false
       | hd' :: tl' -> equal hd hd' && is_prefix equal tl tl')

  (* tail-recursive map *)
  let[@tail_mod_cons] rec safe_map f l = match l with
    | [] -> []
    | x :: xs ->
      f x :: (safe_map[@tailcall]) f xs
    [@@coverage off]

  let align cmp xs ys =
    let next (xs, ys) = match xs, ys with
      | [], [] -> None
      | x::xs', [] -> Some (This x, (xs', []))
      | [], y::ys' -> Some (That y, ([], ys'))
      | x::xs', y::ys' -> (match cmp x y with
        | -1 -> Some (This x, (xs', ys))
        | +1 -> Some (That y, (xs, ys'))
        | _ -> Some (Both(x, y), (xs', ys')))
    in
    Seq.unfold next (xs, ys)
end

module List32 =
struct
  let rec make n x = make' n x []
  and make' n x xs =
    if n = 0l then xs else make' (Int32.sub n 1l) x (x::xs)

  let rec length xs = length' xs 0l
  and length' xs n =
    match xs with
    | [] -> n
    | _::xs' when n < Int32.max_int -> length' xs' (Int32.add n 1l)
    | _ -> failwith "length"

  let rec nth xs n =
    match n, xs with
    | 0l, x::_ -> x
    | n, _::xs' when n > 0l -> nth xs' (Int32.sub n 1l)
    | _ -> failwith "nth"

  let rec take n xs =
    match n, xs with
    | 0l, _ -> []
    | n, x::xs' when n > 0l -> x :: take (Int32.sub n 1l) xs'
    | _ -> failwith "take"

  let rec drop n xs =
    match n, xs with
    | 0l, _ -> xs
    | n, _::xs' when n > 0l -> drop (Int32.sub n 1l) xs'
    | _ -> failwith "drop"
end

module Array =
struct
  include Array

  let rec compare f x y = compare' f x y 0
  and compare' f x y i =
    match i = Array.length x, i = Array.length y with
    | true, true -> 0
    | true, false -> -1
    | false, true -> +1
    | false, false ->
      match f x.(i) y.(i) with
      | 0 -> compare' f x y (i + 1)
      | n -> n

  let for_all2 p xs ys =
    if Array.length xs <> Array.length ys then failwith "for_all2"
    else
      let rec go i =
        i = Array.length xs || (p xs.(i) ys.(i) && go (i + 1))
      in
      go 0
end

module Array32 =
struct
  let make n x =
    if n < 0l || Int64.of_int32 n > Int64.of_int max_int then
      raise (Invalid_argument "Array32.make");
    Array.make (Int32.to_int n) x

  let length a = Int32.of_int (Array.length a)

  let index_of_int32 i =
    if i < 0l || Int64.of_int32 i > Int64.of_int max_int then -1 else
    Int32.to_int i

  let get a i = Array.get a (index_of_int32 i)
  let set a i x = Array.set a (index_of_int32 i) x
  let blit a1 i1 a2 i2 n =
    Array.blit a1 (index_of_int32 i1) a2 (index_of_int32 i2) (index_of_int32 n)
end

module Bigarray =
struct
  open Bigarray

  module Array1_64 =
  struct
    let create kind layout n =
      if n < 0L || n > Int64.of_int max_int then
        raise (Invalid_argument "Bigarray.Array1_64.create");
      Array1.create kind layout (Int64.to_int n)

    let dim a = Int64.of_int (Array1.dim a)

    let index_of_int64 i =
      if i < 0L || i > Int64.of_int max_int then -1 else
      Int64.to_int i

    let get a i = Array1.get a (index_of_int64 i)
    let set a i x = Array1.set a (index_of_int64 i) x
    let sub a i n = Array1.sub a (index_of_int64 i) (index_of_int64 n)
  end
end

module Option =
struct
  let get o x = Option.value o ~default:x

  let exists f o = Option.to_list o |> StdList.exists f

  let map2 f a b =
    match a, b with
    | Some a, Some b -> Some (f a b)
    | _ -> None

  module Syntax =
  struct
    let (let+) x f = Option.map f x
    let (and+) x y = match x, y with
      | Some x, Some y -> Some (x, y)
      | _  -> None
    let (let*) = Option.bind
  end
end

module Promise =
struct
  type 'a t = 'a option ref

  exception Promise

  let make () = ref None
  let make_fulfilled x = ref (Some x)
  let fulfill p x = if !p = None then p := Some x else raise Promise
  let is_fulfilled p = !p <> None
  let value_opt p = !p
  let value p = match !p with Some x -> x | None -> raise Promise
  let lazy_value p f =
    begin
      if not (is_fulfilled p) then
      let x = f () in
      (* Evaluating f might have actually fulfilled this. We assume f to be pure
         (or at least be idempotent), and do not try to update it again.
      *)
      if not (is_fulfilled p) then fulfill p x
    end;
    value p
end

module AllocOnUse =
struct
  (*
  A slighty more elaborate form of a promise: It describes something that can
  be allocated, defined, and used (e.g. a Wasm function with a function id). It
  will only be allocated if it is both defined and used. Cyclic use is supported,
  e.g. the code that defines the thing will already be able to use it.

  Beware: Calling def twice is allowed, the second one will be ignored.

  Re `… Lazy.t` vs. `unit -> …`:
  We use `… Lazy.t` for thunks that will be called exactly once (e.g. producing the
  definition), but `unit -> …` for functions called many times with different
  results (e.g. allocation)
  *)

  type ('a, 'b) alloc = unit -> ('a * ('b -> unit))

  type ('a, 'b) t' =
    | UnUsedUnDef of ('a, 'b) alloc
    | UsedUnDef of 'a * ('b -> unit)
    | UnUsedDef of ('a, 'b) alloc * ('b Lazy.t)
    | UsedDef of 'a
  type ('a, 'b) t = ('a, 'b) t' ref

  let make : ('a, 'b) alloc -> ('a, 'b) t =
    fun alloc -> ref (UnUsedUnDef alloc)

  let def : ('a, 'b) t -> ('b Lazy.t) -> unit =
    fun r mk -> match !r with
      | UnUsedUnDef alloc ->
        r := UnUsedDef (alloc, mk)
      | UsedUnDef (a, fill) ->
        r := UsedDef a;
        fill (Lazy.force mk);
      | UnUsedDef _ | UsedDef _ ->
        ()

  let use : ('a, 'b) t -> 'a =
    fun r -> match !r with
      | UnUsedUnDef alloc ->
        let (a, fill) = alloc () in
        r := UsedUnDef (a, fill);
        a
      | UsedUnDef (a, fill) ->
        a
      | UnUsedDef (alloc, mk) ->
        let (a, fill) = alloc () in
        r := UsedDef a;
        fill (Lazy.force mk);
        a
      | UsedDef a ->
        a
end

module FilePath =
struct
  let segments p = String.split p '/'

  let normalise file_path =
    if file_path = "" then "" else
    let has_trailing_slash =
      Stdlib.Option.is_some (String.chop_suffix "/" file_path) in
    let has_leading_slash = not (Filename.is_relative file_path) in
    let acc = Stack.create () in
    segments file_path |> Stdlib.List.iter
     (function
      | "" -> ()
      | "." -> ()
      | ".." ->
         if Stack.is_empty acc || Stack.top acc = ".."
         then Stack.push ".." acc
         else ignore (Stack.pop acc)
      | segment -> Stack.push segment acc);
    let result = Stack.fold (fun x y -> y ^ "/" ^ x) "" acc in
    if result = ""
    then
      (if has_leading_slash then "/" else
      (if has_trailing_slash then "./" else "."))
    else
      (if has_leading_slash then "/" else "") ^
      (if has_trailing_slash then result
      else Stdlib.Option.get (String.chop_suffix "/" result))

  let relative_to base path =
    String.chop_prefix
      (normalise (base ^ "/"))
      (normalise path)
  let make_absolute base path =
    if not (Filename.is_relative path)
    then path
    else normalise (Filename.concat base path)

  let is_subpath base path =
    if Filename.is_relative base || Filename.is_relative path
    then assert false
    (* We can't just check for prefixing on the string because
       /path/tosomething is not a subpath of /path/to*)
    else List.is_prefix (=) (segments base) (segments path)

  (* TODO: this function does not belong here *)
  (* When opening is successful, but there is a case mismatch (because the file
     system is case insensitive), generate a warning. *)
  let open_in path : in_channel * string list =
    let ic = Stdlib.open_in path in
    let dir, base = Filename.(dirname path, basename path) in
    (* TODO: we could check dir too, but it's hairier *)
    let files = Sys.readdir dir in
    if not (Array.exists (fun name -> name = base) files) then
      begin
        let open Stdlib.String in
        let lbase = lowercase_ascii base in
        if Array.exists (fun name -> lowercase_ascii name = lbase) files then
          let message = Printf.sprintf "file %s has been located with a name of different case" base in
          ic, [message]
        else
          let message = Printf.sprintf "file %s has been located with a different name" base in
          ic, [message]
      end
    else ic, []

    let contents file = In_channel.with_open_bin file In_channel.input_all
end


[@@@warning "-60-32"]
module Test =
struct
(* need to put tests in this file because
   dune does not like it if other files in lib depend on Lib
   Maybe we should break up lib into its components, now that
   it is a dune library.
*)
  let%test "bytes_of_hex DEADBEEF" =
    Hex.bytes_of_hex "DEADBEEF" = "\xDE\xAD\xBE\xEF"
  let%test "bytes_of_hex 0000" =
    Hex.bytes_of_hex "0000" = "\x00\x00"
  let%test "bytes_of_hex empty" =
    Hex.bytes_of_hex "" = ""

  let%test "int_of_hex_byte 00" = Hex.int_of_hex_byte "00" = 0
  let%test "int_of_hex_byte AB" = Hex.int_of_hex_byte "AB" = 0xAB
  let%test "int_of_hex_byte FF" = Hex.int_of_hex_byte "FF" = 0xFF

  (* see https://crccalc.com/ *)
  let %test "crc8 DEADBEEF" = CRC.crc8 "\xDE\xAD\xBE\xEF" = 0xCA
  let %test "crc8 empty" = CRC.crc8 "" = 0x00
  let %test "crc8 0000" = CRC.crc8 "\x00\x00" = 0x00

  let%test "Base32.decode empty" = Base32.decode "" = Ok ""
  let%test "Base32.decode 0000000000" = Base32.decode "AAAAAAA" = Ok "\x00\x00\x00\x00"
  let%test "Base32.decode 000000000000" = Base32.decode "AAAAAAAA" = Ok "\x00\x00\x00\x00\x00"
  let%test "Base32.decode DEADBEEF" = Base32.decode "32W353Y" = Ok "\xDE\xAD\xBE\xEF"

  let%test "Utf8.decode prim emoji" = Wasm.Utf8.encode (Utf8.decode "mo:⛔") = "mo:⛔"
  let%test "Utf8.encode prim emoji" = Utf8.encode (Wasm.Utf8.decode "mo:⛔") = "mo:⛔"

  let%test "Utf8.encode decode large" =
    (* this test overflows if we use Wasm.Utf8 functions instead *)
    let b = Buffer.create 16 in
    for i = 0 to 65535*16 do
      Buffer.add_char b (Char.chr (i land 0x7F))
    done;
    let s = Buffer.contents b in
    Utf8.is_valid s &&
    Utf8.encode (Utf8.decode s) = s

  let%test "Utf8.is_valid agrees with Utf8.decode for single-byte strings" =
    let rec loop f i =
      if i > 0xFF then true
      else if not (f i) then false
      else loop f (i + 1)
    in loop (fun i ->
      let s = Utf8.encode [i] in
      Utf8.is_valid s = (try (ignore (Utf8.decode s); true) with Utf8.Utf8 -> false)
    ) 0

  let%test "String.split \"\"" = String.split "" '/' = [""]
  let%test "String.split \"/\"" = String.split "/" '/' = ["";""]
  let%test "String.split \"//\"" = String.split "//" '/' = ["";"";""]
  let%test "String.split \"a/b/c\"" = String.split "a/b/c" '/' = ["a";"b";"c"]
  let%test "String.split \"a/b//c\"" = String.split "a/b//c" '/' = ["a";"b";"";"c"]

  (* FilePath tests *)
  let normalise_test_case input expected =
    let actual = FilePath.normalise input in
    Stdlib.String.equal actual expected ||
      (Printf.printf
         "\nExpected: %s\nActual: %s\n"
         expected
         actual;
       false)

  let relative_to_test_case root contained expected =
    let actual = FilePath.relative_to root contained in
    let show = function
      | None -> "None"
      | Some s -> "Some " ^ s in
    Stdlib.Option.equal Stdlib.String.equal actual expected ||
      (Printf.printf
         "\nExpected: %s\nActual: %s\n"
         (show expected)
         (show actual);
       false)

  let%test "it removes leading current directory" =
    normalise_test_case "./ListClient.mo" "ListClient.mo"

  let%test "it removes leading `./` for relative paths" =
    normalise_test_case "./lib/foo" "lib/foo"

  let%test "it removes duplicate `//`s" =
    normalise_test_case ".//lib/foo" "lib/foo"

  let%test "it preserves trailing slashes" =
    normalise_test_case "lib/foo/" "lib/foo/"

  let%test "it combines multiple trailing slashes" =
    normalise_test_case "lib/foo//" "lib/foo/"

  let%test "it drops intermediate references to the `.` directory" =
    normalise_test_case "lib/./foo/" "lib/foo/"

  let%test "it applies parent directory traversals" =
    normalise_test_case "lib/../foo/" "foo/"

  let%test "it keeps parent directory references at the start of a path" =
    normalise_test_case "../foo/lib" "../foo/lib"

  let%test "it keeps multiple parent directory references at the start of a path" =
    normalise_test_case "../../foo/lib" "../../foo/lib"

  let%test "it does everything at once" =
    normalise_test_case "../foo//.././lib" "../lib"

  let%test "it handles absolute paths" =
    normalise_test_case "/foo" "/foo"

  let%test "it handles absolute directory paths" =
    normalise_test_case "/foo/./lib/" "/foo/lib/"

  let%test "it handles ." =
    normalise_test_case "." "."

  let%test "it handles ./" =
    normalise_test_case "./." "."

  let%test "it handles ./" =
    normalise_test_case "./" "./"

  let%test "it handles .//" =
    normalise_test_case ".//" "./"

  let%test "it makes one absolute path relative to another one" =
    relative_to_test_case
      "/home/project"
      "/home/project/src/main.mo"
      (Some "src/main.mo")

  let%test "it's robust in the face of trailing slashes" =
    relative_to_test_case
      "/home/project/"
      "/home/project/src/main.mo"
      (Some "src/main.mo")

  let%test "it makes a file path relative to a path" =
    relative_to_test_case
      "/home/project"
      "/home/project/main.mo"
      (Some "main.mo")

  let%test "it preserves trailing slashes" =
    relative_to_test_case
      "/home/project/"
      "/home/project/src/"
      (Some "src/")

  let%test "it handles directory traversals" =
    relative_to_test_case
      "/home/project"
      "/home/project/src/../lib/"
      (Some "lib/")

  let%test "it fails to make disjoint paths relative to one another" =
    relative_to_test_case
      "/home/project"
      "/home/main.mo"
      None

  let%test "it handles relative paths" =
    relative_to_test_case
      "project/src"
      "project/src/Main.mo"
      (Some "Main.mo")
end
