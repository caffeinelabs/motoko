//MOC-FLAG -A=M0194
// warning: compiler crash only manifests itself with
// moc -c issue-2158.mo
// because run.sh -d won't compile if --check fails (AFAICT)
import T "issue-2158/Types";
import C "issue-2158/C";

actor a {
  public shared func f () : async() {
    ignore await C.C();
  }
}
