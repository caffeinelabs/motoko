persistent actor {
  transient let SubnetManager : actor {
    raw_rand() : async Blob;
  } = actor "aaaaa-aa";

  public func random_bytes() : async Blob {
    let bytes = await SubnetManager.raw_rand();
    bytes;
  };
};
