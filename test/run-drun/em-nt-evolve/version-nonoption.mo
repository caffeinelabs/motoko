//MOC-FLAG --enhanced-orthogonal-persistence --enhanced-migration em-nt-evolve/migrations_nonoption --package native-table ../../lib/native-table

import T "Types";

// Refused at upgrade: orders gets vip, a field that is not an option
actor {
  let orders : T.Orders<T.OrderV2Vip>;
  let few : T.Orders<T.OrderV2>;
  let names : T.Orders<Text>;
  let stale : T.Orders<T.OrderV1>
}
