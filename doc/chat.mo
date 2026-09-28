type List<T> = ?{ head : T; var tail : List<T> };

type Post = shared Text -> ();

actor Server = {
  private var clients : List<Client> = null;

  private shared func broadcast(message : Text) {
    var next = clients;
    loop {
      switch next {
        case null { return }
        case ?l {
          l.head.send(message);
          next := l.tail;
        }
      };
    };
  };

  public func subscribe(client : Client) : async Post {
    let cs = { head = client; var tail = clients };
    clients :=  ?cs;
    return broadcast;
  };
};

type Server = actor { subscribe : Client -> async Post; };

actor class Client() = this {
  private var name : Text = "";

  public func start(n : Text, s : Server) {
    name := n;
    let _ = async {
      let post = await s.subscribe(this);
      post("hello from " # name);
      post("goodbye from " # name);
    }
  };

  public func send(msg : Text) {
    debugPrint(name # " received " # msg # "\n");
  };
};

let bob = Client();
let alice = Client();
let charlie = Client();

bob.start("bob", Server);
alice.start("alice", Server);
charlie.start("charlie", Server);
