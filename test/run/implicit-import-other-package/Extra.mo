/// Imports base/Text so it gets loaded (tagged as package `base`) without the
/// main test importing it directly, letting it compete with core/Text.

import Text "mo:base/Text";

module {
  public func touch() { ignore Text.compare("a", "b") };
}
