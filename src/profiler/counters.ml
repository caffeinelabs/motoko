(**

Counters for regions, for super-simple profiling support.

Useful for debugging combinatoric algorithms
or other algorithms with known complexity,
such as search, sorting, etc.

*)

open Source
module Value = Mo_values.Value
module T = Mo_types.Type

type t = {
    label  : ((region * string), int) Hashtbl.t ;
    region : (region, int) Hashtbl.t ;
  }

let dump_count = ref 0

let zeros () = {
    label  = Hashtbl.create 100 ;
    region = Hashtbl.create 100 ;
  }

let bump_region c reg =
  if !ProfilerFlags.profile then
    match Hashtbl.find_opt c.region reg with
      Some n -> Hashtbl.replace c.region reg (n + 1)
    | None   -> Hashtbl.replace c.region reg 1

let bump_label c reg lab =
  if !ProfilerFlags.profile then
    match Hashtbl.find_opt c.label (reg, lab) with
      Some n -> Hashtbl.replace c.label (reg, lab) (n + 1)
    | None   -> Hashtbl.replace c.label (reg, lab) 1

(* lexicographic on (left.file, left.line, left.column, right.line, right.column) *)
let region_order rega regb =
  if rega.left.file = regb.left.file then
    if rega.left.line = regb.left.line then
      if rega.left.column = regb.left.column then
        if rega.right.line = regb.right.line then
          compare rega.right.column regb.right.column
        else
          compare rega.right.line regb.right.line
      else
        compare rega.left.column regb.left.column
    else
      compare rega.left.line regb.left.line
  else
    compare rega.left.file regb.left.file

let label_order laba labb =
  compare laba labb

let dump (c:t) (ve: Value.value Value.Env.t) =
  if !ProfilerFlags.profile then
    if !ProfilerFlags.profile_verbose then (
      Printf.printf "{\n" ;
      Value.Env.iter (fun fn fv ->
          Printf.printf " %s = %s;\n"
            fn (Value.string_of_val 0 T.Non fv)
        )
        ve ;
      Printf.printf "}\n"
    ) ;
    let dump_count = begin
        let d = !dump_count in
        dump_count := d + 1;
        d
      end
    in
    (* Include all labeled regions in the final table: *)
    let labeled_counts =
      Hashtbl.fold (
          fun (reg, label) count counts ->
          ((reg, Some label), count) :: counts)
        c.label []
    in
    (* Include all other regions in the final table: *)
    let all_region_counts =
      Hashtbl.fold (
          fun reg count counts ->
          ((reg, None), count) :: counts)
        c.region labeled_counts
    in
    let sorted_counts =
      List.sort (
          (* Final ordering:
             - counts; bigger first; this is the main ordering constraint.
             - labels; labeled expressions before unlabeled
             - regions; earlier/outer regions before later/enclosed ones
           *)
          fun
            ((rega, laba), x)
            ((regb, labb), y)
          ->
          let diff = x - y in
          if diff <> 0 then -diff else
            match (laba, labb) with
              (Some _, None) -> -1
            | (None, Some _) ->  1
            | (Some _, Some _) -> label_order laba labb
            | (None, None) -> region_order rega regb
        ) all_region_counts
    in
    let file = open_out (!ProfilerFlags.profile_file) in
    let (suffix, flds) =
      (* the suffix of the line consists of field values for each field in `profile_field_names`: *)
      List.fold_right
        (fun var (line, flds) ->
          match Value.Env.find_opt var ve with
            None   -> (Printf.sprintf "%s, #err" line, (var :: flds))
          | Some v -> (Printf.sprintf "%s, %s" line (Value.string_of_val 0 T.Non v), var :: flds)
        ) !ProfilerFlags.profile_field_names ("", [])
    in
    Printf.fprintf file "# column: source region\n" ;
    Printf.fprintf file "# column: source region count\n" ;
    List.iter (fun fld -> Printf.fprintf file "# column: --profile-field: %s\n" fld)
      (List.rev flds) ;
    let lab_total = ref 0 in
    let unlab_total = ref 0 in
    List.iter (fun ((region, labop), region_count) ->
        assert (dump_count = 0);
        (match labop with
           None   -> unlab_total := !unlab_total + region_count
         | Some x -> lab_total := !lab_total + region_count
        );
      ) sorted_counts;
    Printf.fprintf file "# count total (unlabeled): %d\n" !unlab_total ;
    Printf.fprintf file "# ...             labeled: %d\n" !lab_total ;
    List.iter (fun ((region, labop), region_count) ->
        assert (dump_count = 0);
        Printf.fprintf file "%s\"%s\", %s, %d%s\n"
          (!ProfilerFlags.profile_line_prefix)
          (string_of_region region)
          (match labop with
             None   -> "null"
           | Some x -> Printf.sprintf "?\"%s\"" x
          )
          region_count
          suffix
      ) sorted_counts;
    close_out file
