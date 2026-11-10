//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_scalar --package native-table ../../lib/native-table

import T "Types";

// Refused at upgrade: names gets a record row in place of its scalar row
actor {
  let orders : T.Orders<T.OrderV2>;
  let few : T.Orders<T.OrderV2>;
  let names : T.Orders<T.Name>;
  let stale : T.Orders<T.OrderV1>
}
