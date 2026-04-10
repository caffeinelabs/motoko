let magic = "MOIC"
let version = 1

let binary_hash =
  try Digest.to_hex (Digest.file Sys.executable_name)
  with Sys_error _ -> Digest.to_hex (Digest.string Source_id.id)

let binary_hash_len = String.length binary_hash

type cache_data = {
  decs : Ir_def.Ir.dec list;
  id_stamps : (string * int) list;
}

let mkdir_p dir =
  let rec go d =
    if Sys.file_exists d then ()
    else begin go (Filename.dirname d);
      (try Sys.mkdir d 0o755 with Sys_error _ -> ()) end
  in go dir

let write file ~dep_hash (data : cache_data) =
  mkdir_p (Filename.dirname file);
  let tmp = file ^ ".tmp" in
  let oc = open_out_bin tmp in
  (try
    Fun.protect ~finally:(fun () -> close_out oc) (fun () ->
      output_string oc magic;
      output_binary_int oc version;
      output_string oc binary_hash;
      output_string oc dep_hash;
      Marshal.to_channel oc data [Marshal.Closures]
    );
    Sys.rename tmp file
  with exn ->
    (try Sys.remove tmp with Sys_error _ -> ());
    raise exn)

let read file ~dep_hash : cache_data option =
  if not (Sys.file_exists file) then None
  else
    try
      let ic = open_in_bin file in
      Fun.protect ~finally:(fun () -> close_in ic) (fun () ->
        let magic' = really_input_string ic (String.length magic) in
        let version' = input_binary_int ic in
        let compiler' = really_input_string ic binary_hash_len in
        let dep' = really_input_string ic (String.length dep_hash) in
        if magic' <> magic || version' <> version
           || compiler' <> binary_hash || dep' <> dep_hash
        then None
        else Some (Marshal.from_channel ic : cache_data))
    with Sys_error _ | End_of_file | Failure _ -> None

let moic_path ~cache_dir ~dep_hash =
  Filename.concat cache_dir (dep_hash ^ ".moic")
