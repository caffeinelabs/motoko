/// Stub: a base module that pulls base/Text into the environment,
/// so base/Text becomes a loaded (but unimported) library elsewhere.

import Text "Text";

module {
  public func touch() { ignore Text.compare("a", "b") };
}
