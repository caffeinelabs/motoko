#define export __attribute__ ((visibility("default")))

/* Terminal callee. The body doesn't matter — the test only checks
   whether the linker rewrites $foo's `call $bar` to `call $quux`. */
export int quux(int clos, int arg) {
  (void) clos;
  return arg + 1;
}
