open Mo_config

type row = {
  phase : string;
  unit_name : string;
  duration_ns : int64;
}

let rows : row list ref = ref []

let clear () = rows := []

let log_phase heading name =
  if !Flags.verbose then Printf.printf "-- %s %s:\n%!" heading name

let record heading name t0 t1 =
  let ns =
    Int64.of_float ((t1 -. t0) *. 1_000_000_000.)
  in
  rows := { phase = heading; unit_name = name; duration_ns = ns } :: !rows

let with_phase heading name thunk =
  log_phase heading name;
  match !Flags.emit_compiler_timings with
  | None -> thunk ()
  | Some _ ->
    let t0 = Unix.gettimeofday () in
    let r = thunk () in
    let t1 = Unix.gettimeofday () in
    record heading name t0 t1;
    r

let with_phase_diag heading name f =
  log_phase heading name;
  match !Flags.emit_compiler_timings with
  | None -> f ()
  | Some _ ->
    let t0 = Unix.gettimeofday () in
    let r = f () in
    let t1 = Unix.gettimeofday () in
    record heading name t0 t1;
    r

let json_of_row r =
  `Assoc [
    "phase", `String r.phase;
    "unit", `String r.unit_name;
    "duration_ns", `String (Int64.to_string r.duration_ns);
  ]

let maybe_write () =
  match !Flags.emit_compiler_timings with
  | None -> ()
  | Some path ->
    let phases = List.rev !rows |> List.map json_of_row in
    let json =
      `Assoc [
        "schema_version", `Int 1;
        "phases", `List phases;
      ]
    in
    let oc = open_out_bin path in
    Fun.protect
      ~finally:(fun () -> close_out oc)
      (fun () ->
        Yojson.Basic.pretty_to_channel ~std:false oc json;
        output_char oc '\n')
