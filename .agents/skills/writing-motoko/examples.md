# Motoko Examples

Complete working examples demonstrating modern Motoko patterns. All examples verified with moc 1.5.0.

> **Heads-up — enhanced migration:** the actor examples below declare fields with initializers (`let users = List.empty(); var nextId = 0;`). Under `--enhanced-migration`, actor fields **cannot** have initializers — declare them as `var nextId : Nat;` and set initial values in the migration file that introduces them. See the `migrating-motoko-enhanced` skill.

## Principled Architecture

### types.mo

```motoko
module {
  public type UserId = Principal;

  public type User = {
    id : UserId;
    var username : Text;
    var bio : Text;
    var isActive : Bool;
  };

  public type UserPublic = {
    id : UserId;
    username : Text;
    bio : Text;
    isActive : Bool;
  };

  public type Post = {
    id : Nat;
    author : User;
    var title : Text;
    var content : Text;
    var published : Bool;
  };

  public type PostPublic = {
    id : Nat;
    authorId : Principal;
    title : Text;
    content : Text;
    published : Bool;
  };
};
```

### lib/User.mo

```motoko
import Types "../types";

module {
  public type User = Types.User;

  public func new(id : Types.UserId, username : Text) : User {
    { id; var username; var bio = ""; var isActive = true };
  };

  public func updateBio(self : User, newBio : Text) {
    if (newBio.size() > 280) return;
    self.bio := newBio;
  };

  public func ban(self : User) { self.isActive := false };

  public func isValid(self : User) : Bool {
    self.username.size() > 0 and self.isActive;
  };

  public func toPublic(self : User) : Types.UserPublic {
    { id = self.id; username = self.username; bio = self.bio; isActive = self.isActive };
  };
};
```

### lib/Post.mo

```motoko
import Types "../types";

module {
  public type Post = Types.Post;

  public func new(id : Nat, author : Types.User, title : Text) : Post {
    { id; author; var title; var content = ""; var published = false };
  };

  public func publish(self : Post) {
    if (self.content.size() > 0) { self.published := true };
  };

  public func setContent(self : Post, content : Text) {
    self.content := content;
  };
};
```

### mixins/Auth.mo

```motoko
import Types "../types";
import UserLib "../lib/User";
import List "mo:core/List";

mixin (users : List.List<Types.User>) {

  func findUser(p : Principal) : ?Types.User {
    users.find(func(u) { u.id == p });
  };

  public shared ({ caller }) func register(username : Text) : async Bool {
    switch (findUser(caller)) {
      case (?_) return false;
      case (null) {
        users.add(UserLib.new(caller, username));
        return true;
      };
    };
  };

  public shared query ({ caller }) func getProfile() : async ?Types.UserPublic {
    switch (findUser(caller)) {
      case (?user) { ?user.toPublic() };
      case (null) { null };
    };
  };

  public shared ({ caller }) func updateBio(newBio : Text) : async Bool {
    switch (findUser(caller)) {
      case (null) false;
      case (?user) { user.updateBio(newBio); true };
    };
  };
};
```

### mixins/Blog.mo

```motoko
import Types "../types";
import PostLib "../lib/Post";
import List "mo:core/List";
import Runtime "mo:core/Runtime";

mixin (
  users : List.List<Types.User>,
  posts : List.List<Types.Post>,
) {

  public shared ({ caller }) func createPost(title : Text) : async Nat {
    let author = switch (users.find(func(u) { u.id == caller })) {
      case (?u) u;
      case (null) { Runtime.trap("User not registered") };
    };
    let pid = posts.size();
    posts.add(PostLib.new(pid, author, title));
    pid;
  };

  public shared ({ caller }) func publishPost(postId : Nat) : async Bool {
    switch (posts.find(func(p) { p.id == postId })) {
      case (null) false;
      case (?post) {
        if (post.author.id != caller) { return false };
        post.publish();
        true;
      };
    };
  };

  public query func getAllPosts() : async [Types.PostPublic] {
    posts.map<Types.Post, Types.PostPublic>(
      func(p) { { id = p.id; authorId = p.author.id; title = p.title; content = p.content; published = p.published } }
    ).toArray();
  };
};
```

### main.mo

```motoko
import List "mo:core/List";
import Types "types";
import AuthMixin "mixins/Auth";
import BlogMixin "mixins/Blog";

actor Main {
  let users = List.empty<Types.User>();
  let posts = List.empty<Types.Post>();

  include AuthMixin(users);
  include BlogMixin(users, posts);
};
```

## Iterator Chaining

Note: `import Array` enables `.find()`, `.any()`, `.all()` on arrays; `import Iter` enables `.map()`, `.filter()` on iterators; `import Bool` enables `.toText()` on booleans; `import Nat` enables `.toText()` on natural numbers.

```motoko
import Array "mo:core/Array";
import Bool "mo:core/Bool";
import Iter "mo:core/Iter";
import Nat "mo:core/Nat";

actor {
  let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

  public query func demonstrateIterators() : async Text {
    var output = "";

    let doubled = numbers.values().map(func x = x * 2).filter(func x = x > 10).toArray();
    output := output # "Doubled > 10: " # doubled.toText() # "\n";

    let sum = numbers.values().foldLeft(0, func(acc, x) = acc + x);
    output := output # "Sum: " # sum.toText() # "\n";

    switch (numbers.find(func x = x > 5)) {
      case (?found) { output := output # "Found: " # found.toText() # "\n" };
      case (null) {};
    };

    let hasLarge = numbers.any(func x = x > 8);
    let allPositive = numbers.all(func x = x > 0);
    output := output # "Has large: " # hasLarge.toText() # "\n";
    output := output # "All positive: " # allPositive.toText() # "\n";

    output;
  };
};
```

## Map with Custom Key Types

```motoko
import Map "mo:core/Map";
import Order "mo:core/Order";
import Int "mo:core/Int";

actor {
  type Point = { x : Int; y : Int };

  module Point {
    public func compare(a : Point, b : Point) : Order.Order {
      switch (Int.compare(a.x, b.x)) {
        case (#equal) { Int.compare(a.y, b.y) };
        case (other) { other };
      };
    };
  };

  let pointMap = Map.empty<Point, Text>();

  public func addPoint(x : Int, y : Int, pointLabel : Text) : async () {
    pointMap.add({ x; y }, pointLabel);
  };

  public query func getPoint(x : Int, y : Int) : async ?Text {
    pointMap.get({ x; y });
  };
};
```

## Timer with Periodic Cleanup

```motoko
import Timer "mo:core/Timer";
import Time "mo:core/Time";
import List "mo:core/List";

actor {
  let logs = List.empty<(Int, Text)>();
  var timerId : Nat = 0;

  public func startCleanup() : async () {
    timerId := Timer.recurringTimer<system>(
      #seconds(3600),
      func() : async () {
        let oneHourAgo = Time.now() - 3_600_000_000_000;
        let recent = logs.filter(func(timestamp, _) { timestamp > oneHourAgo });
        logs.clear();
        logs.addAll(recent.values());
      },
    );
  };

  public func stopCleanup() : async () {
    Timer.cancelTimer(timerId);
  };
};
```

## Shared Type Boundary

```motoko
import List "mo:core/List";
import Principal "mo:core/Principal";
import Set "mo:core/Set";
import Time "mo:core/Time";

actor {
  type PhotoInternal = {
    id : Nat;
    url : Text;
    uploadedBy : Principal;
    likedBy : Set.Set<Principal>;
    createdAt : Int;
  };

  type Photo = {
    id : Nat;
    url : Text;
    uploadedBy : Text;
    likedBy : [Principal];
    createdAt : Int;
  };

  let photos = List.empty<PhotoInternal>();

  func toPublic(self : PhotoInternal) : Photo {
    {
      self with
      uploadedBy = self.uploadedBy.toText();
      likedBy = Set.toArray(self.likedBy);
    };
  };

  public shared ({ caller }) func upload(url : Text) : async Nat {
    let id = photos.size();
    photos.add({
      id;
      url;
      uploadedBy = caller;
      likedBy = Set.empty<Principal>();
      createdAt = Time.now();
    });
    id;
  };

  public query func getPhotos() : async [Photo] {
    photos.map<PhotoInternal, Photo>(func(p) { toPublic(p) }).toArray();
  };
};
```

## In-Place Mutation Patterns

Use `find` + direct field mutation for updating a single item. Use `mapInPlace` when transforming all items:

```motoko
import List "mo:core/List";

actor {
  type Todo = { id : Nat; text : Text; var completed : Bool };

  let todos = List.empty<Todo>();
  var nextId : Nat = 0;

  public func addTodo(text : Text) : async Nat {
    let id = nextId;
    nextId += 1;
    todos.add({ id; text; var completed = false });
    id;
  };

  public func toggleTodo(targetId : Nat) : async Bool {
    switch (todos.find(func(t) { t.id == targetId })) {
      case (?todo) { todo.completed := not todo.completed; true };
      case (null) false;
    };
  };

  public func completeAll() : async () {
    todos.mapInPlace(func(todo) { { todo with var completed = true } });
  };
};
```

## Type Conversions

Requires `import Nat "mo:core/Nat"`, `import Int "mo:core/Int"`, etc. for dot-notation methods.

```motoko
// Nat ↔ Int
let n : Nat = 42;
let i : Int = n.toInt();
let backToNat = Int.abs(i);

// Nat size widening: Nat8 → Nat16 → Nat32 → Nat64
let nat8 : Nat8 = 255;
let nat16 = nat8.toNat16();
let nat32 = nat16.toNat32();
let nat64 = nat32.toNat64();
let backToNat8 = Nat8.fromNat64(nat64);

// Int size widening: Int8 → Int16 → Int32 → Int64
let int8 : Int8 = -128;
let int16 = int8.toInt16();
let int32 = int16.toInt32();
let int64 = int32.toInt64();
let backToInt8 = Int8.fromInt64(int64);

// To/from Text
let text = n.toText();               // "42"
let maybeNat = Nat.fromText("42");   // : ?Nat
let maybeInt = Int.fromText("-5");   // : ?Int

// To Float
let f = n.toFloat();

// Time is Int (nanoseconds)
let timestamp = Time.now();          // requires import Time "mo:core/Time"
let milliseconds = timestamp / 1_000_000;
```
