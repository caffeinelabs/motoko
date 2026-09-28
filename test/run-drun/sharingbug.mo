type post = shared Text -> async ();

type IServer = actor {
  post: Text -> async ();
  subscribe: IClient -> async post;
};

type IClient = actor {
  send: shared Text -> async ();
};
