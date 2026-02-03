open Mo_config

include Args

(* Warning and diagnostic levels *)
let validate_warning_code code =
  code <> "" &&
  List.exists (fun (c, _, _) -> String.equal c code) Error_codes.warning_codes

let modify_warning_levels level s =
  let codes = String.split_on_char ',' s in
  codes |> List.iter (fun code ->
    if validate_warning_code code then
      Flags.set_warning_level code level
    else
      fail "moc: invalid warning code: %s. Run 'moc --warn-help' to see available warning codes." code)

let warning_args = [
  "--hide-warnings", Arg.Clear Flags.print_warnings, " hide warnings";
  "-Werror", Arg.Set Flags.warnings_are_errors, " treat warnings as errors";
  "-A", Arg.String (modify_warning_levels Flags.Allow),
    "<codes>  disable (allow) comma-separated warning codes, e.g. -A M0194,M0217";
  "-W", Arg.String (modify_warning_levels Flags.Warn),
    "<codes>  enable (warn) comma-separated warning codes, e.g. -W M0223";
  "-E", Arg.String (modify_warning_levels Flags.Error),
    "<codes>  treat as error comma-separated warning codes, e.g. -E M0217";
]
