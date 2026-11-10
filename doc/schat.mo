type List<T> = ?{head : T; var tail : List<T>};

type Post = shared Text -> ();

type Server =
  actor { subscribe : shared (actor { go : shared (Text, Server) -> ();
                                     send : shared (Text) -> ()
				   })
                      -> async Post; };

actor Server = {
  private var clients : List<Client> = null;

  private shared broadcast(message : Text) {
    var next = clients;
    loop {
      switch next {
        case null { return }
        case (?l) {
          l.head.send(message);
          next := l.tail;
        }
      };
    };
  };

  subscribe(client : Client) : async Post {
    let cs = new {head = client; var tail = clients};
    clients := ?cs;
    return broadcast;
  };
};

actor class Client() = this {
  private var name : Text = "";
  go(n : Text , s : Server) {
    name := n;
    let _ = async {
       let post = await s.subscribe(this);
       post("hello from " # name);
       post("goodbye from " # name);
    }
  };
  send(msg : Text) {
    debugPrint(name # " received " # msg # "\n");
  };
};


let bob = Client();
let alice = Client();
let charlie = Client();

bob.go("bob", Server);
alice.go("alice", Server);
charlie.go("charlie", Server);
