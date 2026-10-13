//MOC-FLAG --plugin ../../../_build/compute-plugin-tests/typed
//MOC-FLAG --sanity-checks
import Prim "mo:⛔";

switch (Prim.fixtureRecord("snowman ☃")) {
  case (#err message) { Prim.trap(message) };
  case (#ok value) {
    assert value.value == "snowman ☃";
    assert value.count == 11;
    assert value.children.size() == 257;
    assert value.children[0] == null;
    assert value.children[256] == ?"snowman ☃";
    assert value.pair == (255, true);
    assert value.nested == ?null;
  };
};
assert Prim.fixtureEarlyError("text") == #err("early error");
