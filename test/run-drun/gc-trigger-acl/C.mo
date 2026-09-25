actor class C(gc : actor {
  __motoko_gc_trigger: () -> async ()
})
{

  // attempt to call passed in gc_trigger
  public shared func callback() : async () {
    await gc.__motoko_gc_trigger(); // should fail unless controller or selfaxs
  }
}
