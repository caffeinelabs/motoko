import Prim "mo:⛔";
import S "server";

module {

  public persistent actor class Client() = this {
    // TODO: these should be constructor params once we can compile them
    transient var name : Text = "";
    transient var server : ?S.Server  = null;

    public func go(n : Text, s : S.Server) {
      name := n;
      server := ?s;
      ignore(async {
        let sub = await s.subscribe(this.send);
        sub.post("hello from " # name);
        sub.post("goodbye from " # name);
        sub.cancel();
      })
    };

    public func send(msg : Text) {
      Prim.debugPrint(name # " received " # msg # "\n");
    };
  };

}
