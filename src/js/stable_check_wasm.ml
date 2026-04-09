let read_file filename =
  let ic = open_in filename in
  let n = in_channel_length ic in
  let s = Bytes.create n in
  really_input ic s 0 n;
  close_in ic;
  Bytes.to_string s

let () =
  if Array.length Sys.argv <> 3 then begin
    Printf.eprintf "Usage: %s <pre.most> <post.most>\n" Sys.argv.(0);
    exit 2
  end;
  let pre = read_file Sys.argv.(1) in
  let post = read_file Sys.argv.(2) in
  match Stable_check.stable_compatible pre post with
  | Ok ((), msgs) ->
    List.iter (fun (msg : Diag.message) ->
      Printf.eprintf "%s\n" msg.Diag.text) msgs
  | Error msgs ->
    List.iter (fun (msg : Diag.message) ->
      Printf.eprintf "%s\n" msg.Diag.text) msgs;
    exit 1
