import Mix "import-mixin/Mix";
actor {
  include Mix();
  /// Doc for direct method gamma.
  public shared func gamma() : async Text { "hi" };
};
