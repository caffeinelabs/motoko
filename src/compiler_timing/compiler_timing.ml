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

module Phase_map = Map.Make (String)

(** Sum [duration_ns] for each distinct [phase] heading across all units. *)
let rollup_by_phase (rows : row list) : (string * int64) list =
  let m =
    List.fold_left
      (fun acc r ->
        Phase_map.update r.phase
          (function
            | None -> Some r.duration_ns
            | Some n -> Some (Int64.add n r.duration_ns))
          acc)
      Phase_map.empty rows
  in
  Phase_map.bindings m |> List.sort (fun (a, _) (b, _) -> String.compare a b)

let json_of_phase_total (phase, ns) =
  `Assoc [
    "phase", `String phase;
    "duration_ns", `String (Int64.to_string ns);
  ]

let maybe_write () =
  match !Flags.emit_compiler_timings with
  | None -> ()
  | Some path ->
    let rev = List.rev !rows in
    let phases = List.map json_of_row rev in
    let phase_totals =
      rollup_by_phase rev |> List.map json_of_phase_total
    in
    let json =
      `Assoc [
        "schema_version", `Int 1;
        "phases", `List phases;
        "phase_totals", `List phase_totals;
      ]
    in
    let oc = open_out_bin path in
    Fun.protect
      ~finally:(fun () -> close_out oc)
      (fun () ->
        Yojson.Basic.pretty_to_channel ~std:false oc json;
        output_char oc '\n')
