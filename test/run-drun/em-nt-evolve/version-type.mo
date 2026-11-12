//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_type --package native-table ../../lib/native-table

import T "Types";

// Refused at upgrade: amount changes from Nat to Int
actor {
  let orders : T.Orders<T.OrderV2Int>;
  let few : T.Orders<T.OrderV2>;
  let names : T.Orders<Text>;
  let stale : T.Orders<T.OrderV1>
}
