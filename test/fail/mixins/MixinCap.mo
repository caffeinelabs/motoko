mixin() {

  func f<system>(){};
  f<system>(); // rejected, but maybe could be accepted? Safe for now at least
  await(async {}); // correctly rejected

};
