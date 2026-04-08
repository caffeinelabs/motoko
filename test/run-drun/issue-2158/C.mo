import Types "Types";

shared actor class C() {
  private shared func f() : () { }; // error!
}
