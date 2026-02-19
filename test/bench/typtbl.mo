import { debugPrint; performanceCounter; getCandidTypeLimits; } = "mo:⛔";

// various sizes
import msg256x1 = "blob:file:messages/msg256x1.bin";
import msg256x2 = "blob:file:messages/msg256x2.bin";
import msg512x1 = "blob:file:messages/msg512x1.bin";
import msg768x1 = "blob:file:messages/msg768x1.bin";
import msg1024x1 = "blob:file:messages/msg1024x1.bin";

actor Decoding {
  type H1<T> = [[[[[[[[[[[[[[[[T]]]]]]]]]]]]]]]];
  type H2<T> = H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<H1<T>>>>>>>>>>>>>>>>;

  type Nested256 = H2<Nat8>;
  type Nested512 = H2<H2<Nat8>>;
  type Nested768 = H2<H2<H2<Nat8>>>;
  type Nested1024 = H2<H2<H2<H2<Nat8>>>>;

  debugPrint(debug_show getCandidTypeLimits<system>());

  func measure<T>(f : () -> ?T) {
      let p0 = performanceCounter(0);
      ignore f();
      let p1 = performanceCounter(0);
      debugPrint(debug_show { cycles = p1 - p0 });
  };
  // irrelevant type table
  measure<()>(func() = from_candid msg256x1);
  // relevant type table
  measure<Nested256>(func() = from_candid msg256x1);
  measure<Nested512>(func() = from_candid msg512x1);
  measure<Nested768>(func() = from_candid msg768x1);
  measure<Nested1024>(func() = from_candid msg1024x1);
  // relevant tuple type table
  measure<(Nested256, Nested256)>(func() = from_candid msg256x2);
}
