// Benchmark: ingress/egress round-trip of an Apple Event Object
// Specifier (a non-Candid wire format) through pure-Motoko codec.
//
// This file lands incrementally. v1 (this commit) defines the full
// ObjectSpec/BoolExpr type surface from `.claude/plans/query.md` and
// a builder for the example query
//
//   every client's mean yearly income whose country is Germany
//                  and age between 45 and 55 years
//
// v2 will add AE compact-binary samples generated via macOS
// `osarun`/`osacompile` from real AppleScript expressions, embedded
// here as `Blob` literals. v3 grows a Motoko AE decoder that turns
// those bytes into the variant tree below; v4 the matching encoder;
// v5 wires both into a `(with decoder = …; encoder = …)` parenthetical
// on a public actor method whose body is the identity, so the
// benchmark observes the codec cost on a real ingress/egress path.
//
// At each step the harness reports `payload_bytes`,
// `decoder_heap`/`decoder_cycles`, `encoder_heap`/`encoder_cycles` so the
// per-stage cost is visible as the codec fills in.
//
// Wire-format reference: `.claude/plans/query.md` §"Apple Events
// Compact Binary Encoding".

import {
  performanceCounter;
  debugPrint;
  rts_heap_size;
} = "mo:⛔";

actor {

  // ───────────────────────── core types ─────────────────────────
  // Full surface from `query.md`. The bench will grow more
  // queries and codec coverage as things develop; the type
  // definitions are stable from v1.

  type CandidValue = {
    #null_;
    #bool : Bool;
    #int : Int;
    #int32 : Int32;
    #nat : Nat;
    #text : Text;
    #blob : Blob;
  };

  type Comparison = { #eq; #ne; #lt; #gt; #le; #ge };

  type BoolExpr = {
    #compare : { prop : Text; op : Comparison; value : CandidValue };
    #and_ : (BoolExpr, BoolExpr);
    #or_ : (BoolExpr, BoolExpr);
    #not_ : BoolExpr;
  };

  type KeyForm = {
    #absolutePosition : Int;
    #name : Text;
    #uniqueID : Nat;
    #property : Text;
    #range : (ObjectSpec, ObjectSpec);
    #test : BoolExpr;
  };

  type ObjectSpec = {
    #root;
    #obj : { class_ : Text; container : ObjectSpec; key : KeyForm };
  };

  // ───────────────────── example query builders ─────────────────
  // The catalogue grows over time; each function returns a fully
  // typed ObjectSpec that the codec must round-trip losslessly.

  // Q1: every client's yearly income whose country == "Germany"
  //     and 45 <= age <= 55
  func germanMidlifeClientIncome() : ObjectSpec {
    let cmp_country : BoolExpr = #compare {
      prop = "country"; op = #eq; value = #text "Germany"
    };
    let cmp_age_lo : BoolExpr = #compare {
      prop = "age"; op = #ge; value = #int32 45
    };
    let cmp_age_hi : BoolExpr = #compare {
      prop = "age"; op = #le; value = #int32 55
    };
    let predicate : BoolExpr =
      #and_ (cmp_country, #and_ (cmp_age_lo, cmp_age_hi));
    let clients : ObjectSpec = #obj {
      class_ = "client";
      container = #root;
      key = #test predicate;
    };
    #obj {
      class_ = "property";
      container = clients;
      key = #property "yearlyIncome";
    };
  };

  // ───────────────────────── codec stubs ────────────────────────
  // Filled in incrementally. v3: `decoder`. v4: `encoder`. Until
  // both are real, `go` reports payload_bytes only.

  // ───────────────────────── benchmark harness ──────────────────

  func counters() : (Nat, Nat64) = (rts_heap_size(), performanceCounter(0));

  // Encoder/decoder time themselves and report on each call.  The
  // `stage` label lets the caller distinguish the three legs of a
  // round-trip (pre-encode for fixture, ingress decode, egress
  // re-encode); this also covers the pre-encode that earlier scaffolds
  // left untimed, which will matter once v4 lands a real encoder.
  func encoder(stage : Text, _ : ObjectSpec) : Blob {
    let (h0, c0) = counters();
    let wire : Blob = "" : Blob;  // TODO v4: AE compact-binary encoder.
    let (h1, c1) = counters();
    debugPrint(debug_show {
      stage = "encoder/" # stage;
      heap = (h1 : Int) - h0;
      cycles = c1 - c0;
      bytes = wire.size();
    });
    wire
  };

  func decoder(stage : Text, _ : Blob) : ObjectSpec {
    let (h0, c0) = counters();
    let spec : ObjectSpec = #root;  // TODO v3: AE compact-binary decoder.
    let (h1, c1) = counters();
    debugPrint(debug_show {
      stage = "decoder/" # stage;
      heap = (h1 : Int) - h0;
      cycles = c1 - c0;
    });
    spec
  };

  public func go() : async () {
    let spec = germanMidlifeClientIncome();

    // Three legs, each self-timed: pre-encode produces the wire bytes
    // (will be replaced by an osarun-generated fixture in v2), the
    // ingress decode is the actual ingress codec cost, and the egress
    // re-encode is what the canister would ship back when the body is
    // the identity.
    let wire = encoder("pre", spec);
    let decoded = decoder("ingress", wire);
    let _reencoded = encoder("egress", decoded);
  };
}

//CALL ingress go 0x4449444C0000

//SKIP run
//SKIP run-ir
//SKIP run-low
