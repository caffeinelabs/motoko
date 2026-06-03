(** Cache of source-file content for diagnostic rendering.
    Short-lived by design: long-running processes (moc.js, VSCode) must not see stale content across compilations.
    Create a fresh cache per batch of messages (see [Diag.print_messages]). *)

type t

val create : unit -> t

(** 0-based UTF-8 byte offset of [pos] from the start of [pos.file].
    [None] for synthetic positions or unreadable files. *)
val byte_offset : t -> Source.pos -> int option

(** 0-based codepoint column of [pos] on its line.
    Falls back to [pos.column] (raw byte column) for synthetic positions or unreadable files. *)
val codepoint_column : t -> Source.pos -> int

(** Loaded UTF-8 content of [path]. [None] if unreadable. *)
val content : t -> string -> string option

