type Subscription = {
  post : shared Text -> (); // revokable by Server
  cancel : shared () -> ();
};
