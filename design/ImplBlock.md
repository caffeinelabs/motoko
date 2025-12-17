Impl Blocks
===========

Writing libraries that make use of generics and implicits involves writing rather noisy and repetitive signatures.

For example every function that needs to perform a lookup in `core/Map` needs at least two type parameters, as well as an implicit argument for a compare function.

```
public func get<K, V>(self : Map<K, V>, key : K, compare : (implicit : (K, K) -> Order.Order)) : ?V { ... }
public func insert<K, V>(self : Map<K, V>, key : K, value : V, compare : (implicit : (K, K) -> Order.Order)) : Bool { ... }
public func map<K, V, V1>(self : Map<K, V>, project : (K, V) -> V1, compare : (implicit : (K, K) -> Order.Order)) : Map<K, V1> { ... }
```

This is in stark contrast to the "equivalent" class definition, that needs to define the type parameters, as well as the implicit argument once and
then gets to be very concise for its method definitions.

```
public class Map<K, V>(compare : (implicit : (K, K) -> Order.Order)) {
  public func get(key : K) : ?V { ... }
  public func insert(key : K, value : V) : Bool { ... }
  // Can't actually be defined, because of non-regular recursion
  // public func map<V1>(project : (K, V) -> V1) : Map<K, V1> { ... }
}
```

A proven solution from `Rust` is the `impl` block, which when adapted to Motoko could look something like this:

```
impl <typ_params>? <typ> (using <params>)? { <func_dec>* }
```

- The specified type parameters get prepended to every func_dec
- The specified type becomes the type of the first parameter for every func_dec (Potentially make `self` a conditional keyword inside impl blocks)
- The specified `using` parameters get appended to the argument list of every func_dec (restrict to implicits?)

The following example code would then produce the equivalent definitions as given in the initial problem statement:

```
impl<K, V> Map<K, V> using (compare : (implicit : (K, K) -> Order.Order)) {
  public func get(self, key : K) : ?V { ... }
  public func insert(self, key : K, value : V) : Bool { ... }
  public func map<V1>(self, project : (K, V) -> V1) : Map<K, V1> { ... }
}
```

Allowing multiple `impl` blocks gives more flexibility then the OO version, as functions such as `empty`, `toArray`, ... that don't require the `compare` implicit can be defined in a separate block without the `using` clause, or as standalone definitions as before.
Functions like `map` can be defined just fine and only need to specify the _additional_ type parameters they require.


## Variations/Future extensions

- Could move the `self` name into the impl block header `impl (self : T) {}`, to get even closer to the OO notation. Might give the wrong impression that closures are involved.
- Could allow moving the visibility modifier `public impl ...` which would flip the default visibility inside the impl block to public
- Could expose the type in the impl header as `Self` inside the body
- We don't actually need to restrict the decls inside the impl block to function declarations, it just wouldn't have an effect on other declarations
