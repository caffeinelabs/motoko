//MOC-FLAG --default-persistent-actors --error-format=json
// Verifies the machine-applicable edit for M0218 ("redundant `stable`").
actor {
  stable let _x = #x;  // warn M0218
  stable var _y = #y;  // warn M0218
}
