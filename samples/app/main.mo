import Server "server";
import Client "client";

let server = await Server.Server();
let bob = await Client.Client();
let alice = await Client.Client();
let charlie = await Client.Client();
bob.go("bob", server);
alice.go("alice", server);
charlie.go("charlie", server)
