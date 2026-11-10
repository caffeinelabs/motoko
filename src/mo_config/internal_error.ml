
let setup_handler () =
  let open Printexc in
  record_backtrace true;
  set_uncaught_exception_handler (fun exn rb ->
    Printf.eprintf "OOPS! You've triggered a compiler bug.\n";
    Printf.eprintf "Please report this at https://github.com/caffeinelabs/motoko/issues/new with the following details:\n\nMotoko %s\n\n" Source_id.banner;
    default_uncaught_exception_handler exn rb
  );
