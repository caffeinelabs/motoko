// spike object wrapper: binds the migration function under the name the
// desugarer's object-mode injection looks for (`mco_entry`).
import M "migrations_v2/20250102_000000";

let mco_entry = M.migration;
