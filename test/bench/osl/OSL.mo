// Object Support Library — data-model-agnostic AEOM query engine.
//
// Provides the AE binary codec (encoder + decoder), the Smurf protocol
// (existential entity abstraction), the ObjectSpec walker (resolve/eval),
// and the Lingo self-description types.
//
// Imported by bench canisters (e.g. object-spec.mo) which supply their
// own data-model-specific Smurfs and PropReader tables.
//
// Wave 2 items still in object-spec.mo (depend on ValueSmurf charWrapper):
//   ValueSmurf, VarAccessor<T>, CollectionSmurf<T>, FlattenedSmurf<P,E>

import {
  arrayMutToBlob;
  nat8ToNat;
  nat32ToNat;
  nat32ToInt32;
  int32ToNat32;
  natToNat8;
  intToNat32Wrap;
  intToInt32Wrap;
  abs;
  Array_init;
  Array_tabulate;
  decodeUtf8;
  encodeUtf8;
  trap;
  error;
  charToText;
  charToNat32;
  nat32ToChar;
  natToNat32;
  int32ToInt;
} = "mo:⛔";

module {

  // ── Wire types ────────────────────────────────────────────────────────────

  public type CandidValue = {
    #null_;
    #bool : Bool;
    #int : Int;
    #int32 : Int32;
    #nat : Nat;
    #text : Text;
    #blob : Blob;
    #type_ : Text;   // an OSType / 4cc class-or-property code (Text now, Nat32 later)
    #record : [(Text, CandidValue)];  // AERecord — keyword(4cc)→value, e.g. `id {left:…, right:…}`
    #list : [CandidValue];            // AEList — positional values, e.g. `id {a, b}`
  };

  public type Comparison = { #eq; #ne; #lt; #gt; #le; #ge };

  public type BoolExpr = {
    #compare  : { prop : Text; op : Comparison; value : CandidValue };
    #and_     : (BoolExpr, BoolExpr);
    #or_      : (BoolExpr, BoolExpr);
    #not_     : BoolExpr;
    #always;            // synthetic literal-true; OSL-internal, not wire-encodable
    #contains : { prop : Text; values : [CandidValue] };  // set membership: prop ∈ values
  };

  public type KeyForm = {
    #absolutePosition : Int;
    #name             : Text;
    #uniqueID         : Nat;
    #property         : Text;
    #range            : (ObjectSpec, ObjectSpec);
    #test             : BoolExpr;
    #every;             // OSL resolves via #test accessor with #always
    #id               : CandidValue;  // formUniqueID payload: a scalar (single object by key) or a #record/#list (join spec)
  };

  public type ObjectSpec = {
    #root;
    #obj   : { class_ : Text; container : ObjectSpec; key : KeyForm };
    #value : CandidValue;   // data descriptor (utxt/long/…)
    #list  : [ObjectSpec];  // typeAEList: N nested descriptors
  };

  // ── Predicate evaluation ──────────────────────────────────────────────────

  public func cmp(a : CandidValue, op : Comparison, b : CandidValue) : Bool {
    switch (a, op, b) {
      case (#text x,  #eq, #text y)  x == y;
      case (#text x,  #ne, #text y)  x != y;
      case (#text x,  #lt, #text y)  x <  y;
      case (#text x,  #gt, #text y)  x >  y;
      case (#text x,  #le, #text y)  x <= y;
      case (#text x,  #ge, #text y)  x >= y;
      case (#int32 x, #eq, #int32 y) x == y;
      case (#int32 x, #ne, #int32 y) x != y;
      case (#int32 x, #lt, #int32 y) x <  y;
      case (#int32 x, #gt, #int32 y) x >  y;
      case (#int32 x, #le, #int32 y) x <= y;
      case (#int32 x, #ge, #int32 y) x >= y;
      case (#bool  x, #eq, #bool  y) x == y;
      case (#bool  x, #ne, #bool  y) x != y;
      case (#nat   x, #eq, #nat   y) x == y;
      case (#nat   x, #ne, #nat   y) x != y;
      case (#nat   x, #lt, #nat   y) x <  y;
      case (#nat   x, #gt, #nat   y) x >  y;
      case (#nat   x, #le, #nat   y) x <= y;
      case (#nat   x, #ge, #nat   y) x >= y;
      case _ trap "AE: cmp type mismatch";
    }
  };

  // Generic BoolExpr evaluator.  `lookup prop` returns the reader for the
  // named property on entity `c`.  Only depends on `cmp` above; no bench
  // data model knowledge.
  public func evalPred<T>(lookup : Text -> (T -> CandidValue), e : BoolExpr, c : T) : Bool {
    switch e {
      case (#and_ (a, b)) evalPred(lookup, a, c) and evalPred(lookup, b, c);
      case (#or_  (a, b)) evalPred(lookup, a, c) or  evalPred(lookup, b, c);
      case (#not_ a)      not (evalPred(lookup, a, c));
      case (#always)      true;
      case (#compare { prop; op; value }) cmp(lookup(prop) c, op, value);
      case (#contains { prop; values }) {
        let v = lookup(prop) c;
        label found : Bool do {
          for (candidate in values.vals()) { if (cmp(v, #eq, candidate)) break found true };
          false
        }
      };
    }
  };

  // ── Smurf protocol ────────────────────────────────────────────────────────

  public type Iter<T> = { next : () -> ?T };

  public type LookupKey = {
    #indexed : Int;
    #named   : Text;
    #test    : BoolExpr;
    #id      : CandidValue;   // formUniqueID payload (scalar or #record/#list)
  };

  public type Smurf = {
    class4cc  : Text;
    accessors : [Accessor];
    toDesc    : () -> async* ObjectSpec;
    filter    : BoolExpr -> Smurf;
  };

  public type Accessor = {
    kind   : { #property; #element };   // AEOM split: #property → shows in `properties` (pALL); #element → a collection
    form   : { #indexed; #named; #test; #id };
    fourcc : Text;
    lookUp : (parent : Smurf, key : LookupKey) -> Smurf;
  };

  // Structural prefix of an element accessor (`Accessor` minus `lookUp`), so a
  // literal need only graft the body: `{ indexedElement "ord " with lookUp = func(…) {…} }`.
  // The return annotation is required (an unannotated `= { … }` body parses as a
  // block); it also carries the wide variant types so `{ … with lookUp = … }` is
  // exactly `Accessor`.
  public type ElementProto = { kind : { #property; #element }; form : { #indexed; #named; #test; #id }; fourcc : Text };
  public func indexedElement(fourcc : Text) : ElementProto = { kind = #element; form = #indexed; fourcc };
  public func namedElement(fourcc : Text)   : ElementProto = { kind = #element; form = #named;   fourcc };
  public func testElement(fourcc : Text)    : ElementProto = { kind = #element; form = #test;    fourcc };
  public func idElement(fourcc : Text)      : ElementProto = { kind = #element; form = #id;      fourcc };

  public func notFoundSmurf(parent : Smurf) : Smurf = {
    class4cc  = "";
    accessors = [];
    toDesc    = func() : async* ObjectSpec {
      throw error ("Error (errAENoSuchObject = -1728) in " # debug_show (await* parent.toDesc()))
    };
    filter    = func _ = notFoundSmurf parent;
  };

  // A thin terminal leaf carrying a CandidValue — no accessors, no char
  // navigation (cf. the bench `ValueSmurf`, which grows char accessors for text).
  // Generic collection helpers (e.g. `CollectionSmurf`'s `pcnt` count) wrap their
  // Int result as a leaf via this — keeping `CollectionSmurf` free of the bench
  // `ValueSmurf`.  Singleton, so `filter` traps.
  public func simpleLeaf(v : CandidValue) : Smurf = {
    class4cc  = "";
    accessors = [];
    toDesc    = func() : async* ObjectSpec { #value v };
    filter    = func _ = trap "AE: simpleLeaf.filter — leaves are not filterable";
  };

  public func findAccessor(parent : Smurf, fourcc : Text, form : { #indexed; #named; #test; #id }) : ?Accessor {
    for (a in parent.accessors.vals()) {
      if (a.fourcc == fourcc and a.form == form) return ?a;
    };
    null
  };

  // The universal `properties` (pALL) accessor.  Walks the parent's accessor
  // table, keeps the #property entries, and yields their codes as a typeType
  // list — `properties of X ⇒ {«class …», …}`.  Deliberately NOT stored in any
  // Smurf's `accessors` (so it never lists itself); `resolve` falls back to it.
  // A join `Row` (left+right accessors merged via `smurfMap`) serves this for
  // the whole joined tuple for free.
  public let pAllAccessor : Accessor = {
    kind   = #property;
    form   = #named;
    fourcc = "pALL";
    lookUp = func(parent : Smurf, _ : LookupKey) : Smurf {
      let accs = parent.accessors;
      var n : Nat = 0;
      for (a in accs.vals()) { if (a.kind == #property) n += 1 };
      let codes = Array_init<ObjectSpec>(n, #root);
      var i : Nat = 0;
      for (a in accs.vals()) {
        if (a.kind == #property) { codes[i] := #value (#type_ (a.fourcc)); i += 1 };
      };
      {
        class4cc  = "";
        accessors = [];
        toDesc    = func() : async* ObjectSpec { #list (Array_tabulate<ObjectSpec>(n, func j = codes[j])) };
        filter    = func _ = trap "AE: properties result is not filterable";
      }
    };
  };

  // 1-based positional fold; side-effecting.
  public func iteri<T>(arr : [T], f : (Nat, T) -> ()) {
    var idx : Nat = 0;
    for (item in arr.vals()) { idx += 1; f(idx, item) };
  };

  // Distributive lift over [Smurf]: merges accessor tables, renders #list.
  public func smurfMap(parent : Smurf, elements : [Smurf]) : Smurf {
    var totalAccs : Nat = 0;
    for (e in elements.vals()) totalAccs += e.accessors.size();
    let keyBuf = Array_init<(Text, { #indexed; #named; #test; #id }, { #property; #element })>(totalAccs, ("", #indexed, #element));
    var nKeys : Nat = 0;
    for (e in elements.vals()) {
      for (a in e.accessors.vals()) {
        var k : Nat = 0;
        var seen : Bool = false;
        while (k < nKeys and not seen) {
          let (fcc, fm, _) = keyBuf[k];
          if (fcc == a.fourcc and fm == a.form) seen := true;
          k += 1;
        };
        if (not seen) { keyBuf[nKeys] := (a.fourcc, a.form, a.kind); nKeys += 1 };
      };
    };
    let accessors : [Accessor] = Array_tabulate<Accessor>(nKeys, func i {
      let (fourcc, form, kind) = keyBuf[i];
      {
        kind;
        fourcc;
        form;
        lookUp = func(par : Smurf, key : LookupKey) : Smurf {
          let resBuf = Array_init<Smurf>(elements.size(), notFoundSmurf par);
          var present : Nat = 0;
          for (j in elements.keys()) {
            let elem = elements[j];
            switch (findAccessor(elem, fourcc, form)) {
              case (?acc) { resBuf[present] := acc.lookUp(elem, key); present += 1 };
              case null ();
            };
          };
          smurfMap(par, Array_tabulate<Smurf>(present, func j = resBuf[j]))
        };
      }
    });
    {
      class4cc   = "";
      accessors;
      toDesc     = func() : async* ObjectSpec {
        let buf = Array_init<ObjectSpec>(elements.size(), #root);
        for (i in elements.keys()) buf[i] := await* elements[i].toDesc();
        #list (Array_tabulate<ObjectSpec>(elements.size(), func j = buf[j]))
      };
      filter     = func _ = notFoundSmurf parent;
    }
  };

  // VarAccessor<T>: typed escape hatch over a stable [T]. Captures the
  // collection at construction — no Candid round-trip on input.
  // `wrap : T -> Smurf` is supplied per entity to build the appropriate
  // child Smurf (e.g. a clientSmurf wrapping a Client).
  public class VarAccessor<T>(
    stab    : [T],
    fourcc_ : Text,
    form_   : { #indexed; #named; #test; #id },
    wrap    : (T, Nat, Smurf) -> Smurf,   // position is 1-based slot in `stab`
    getName : T -> Text,             // used when form_ = #named; ignored otherwise
  ) {
    public let kind   = #element;   // a collection-navigation accessor
    public let fourcc = fourcc_;
    public let form   = form_;
    public func lookUp(parent : Smurf, key : LookupKey) : Smurf {
      switch (form_, key) {
        case (#indexed, #indexed i) {
          // AppleScript convention: 1-based forward, negatives count from end
          // (-1 = last, -size = first); out of range → notFound.
          let size = stab.size();
          let n : Nat =
            if (i > 0) abs i
            else if (i < 0 and abs i <= size) size - abs i + 1
            else 0;
          if (n == 0 or n > size) notFoundSmurf parent
          else wrap(stab[n - 1], n, parent)
        };
        case (#named, #named target) {
          // Linear scan; relies on the init-time uniqueness assertion.
          var found : ?T = null;
          var foundAt : Nat = 0;
          var idx : Nat = 0;
          for (item in stab.vals()) {
            idx += 1;
            switch found {
              case null if (getName item == target) { found := ?item; foundAt := idx };
              case _ ();
            };
          };
          switch found {
            case (?item) wrap(item, foundAt, parent);
            case null notFoundSmurf parent;
          }
        };
        case _ notFoundSmurf parent;  // TODO: #test (with matching form_)
      }
    };
  };

  // CollectionSmurf<T>: typed multi-element view over a stable [T]. Holds
  // an accumulated predicate (`null` = unfiltered) so `filter` composes via
  // `#and_`. `toDesc` resolves eagerly into a `#list` of element references —
  // each match is wrapped (e.g. clientSmurf) and asked for its own toDesc.
  public class CollectionSmurf<T>(
    source  : [T],
    classCC : Text,
    parent  : Smurf,
    wrap    : (T, Nat, Smurf) -> Smurf,    // Nat = 1-based source position
    lookup  : Text -> (T -> CandidValue),
    getName : T -> Text,                   // for #named lookup over the filtered view
    pred    : ?BoolExpr,
  ) {
    func cardinality() : Nat {
      var n = 0;
      for (t in source.vals()) {
        let m = switch pred { case null true; case (?p) evalPred(lookup, p, t) };
        if m n += 1;
      };
      n
    };

    func passes(t : T) : Bool =
      switch pred { case null true; case (?p) evalPred(lookup, p, t) };

    // Inherited element accessors (same identity as the parent's
    // VarAccessor<T> for this classCC, but iterating the filtered local
    // view instead of the full stable [T]).  Position math mirrors
    // VarAccessor exactly: 1-based, negative-from-end.
    func indexedLookup(par : Smurf, key : LookupKey) : Smurf =
      switch key {
        case (#indexed i) {
          let total = cardinality();
          let target : Nat =
            if (i > 0) abs i
            else if (i < 0 and abs i <= total) total - abs i + 1
            else 0;
          if (target == 0 or target > total) notFoundSmurf par
          else {
            var seen : Nat = 0;
            var srcIdx : Nat = 0;
            var result : Smurf = notFoundSmurf par;
            label l for (t in source.vals()) {
              srcIdx += 1;
              if (passes t) {
                seen += 1;
                if (seen == target) { result := wrap(t, srcIdx, par); break l };
              };
            };
            result
          }
        };
        case _ notFoundSmurf par;
      };

    func namedLookup(par : Smurf, key : LookupKey) : Smurf =
      switch key {
        case (#named target) {
          var result : Smurf = notFoundSmurf par;
          var srcIdx : Nat = 0;
          label l for (t in source.vals()) {
            srcIdx += 1;
            if (passes t and getName t == target) {
              result := wrap(t, srcIdx, par); break l;
            };
          };
          result
        };
        case _ notFoundSmurf par;
      };

    // Inherited #test: compose predicates via `#and_` and return a
    // refined CollectionSmurf.  Avoids the `self`-at-class-init dance
    // by calling the constructor directly with the AND-composed pred.
    // `parent` of the new collection is THIS collection's parent — keeps
    // the AE-wire navigation chain coherent with successive filters.
    func testLookup(par : Smurf, key : LookupKey) : Smurf =
      switch key {
        case (#test newPred) {
          let combined : ?BoolExpr = switch pred {
            case null ?newPred;
            case (?old) ?(#and_ (old, newPred));
          };
          CollectionSmurf<T>(source, classCC, parent, wrap, lookup, getName, combined)
        };
        case _ notFoundSmurf par;
      };

    public let  class4cc                   = classCC;
    // Inherited element accessors mirror only the (classCC, form) pairs
    // the parent actually exposes — never fabricate.  E.g. characters
    // of a name have positional access but not name-keyed access; a
    // CollectionSmurf<Char> built off a name-valued parent should
    // therefore expose #indexed but not #named.
    let parentHasIndexed : Bool = switch (findAccessor(parent, classCC, #indexed)) { case null false; case _ true };
    let parentHasNamed   : Bool = switch (findAccessor(parent, classCC, #named))   { case null false; case _ true };
    let parentHasTest    : Bool = switch (findAccessor(parent, classCC, #test))    { case null false; case _ true };
    let indexedAcc : Accessor = { kind = #element; form = #indexed; fourcc = classCC; lookUp = indexedLookup };
    let namedAcc   : Accessor = { kind = #element; form = #named;   fourcc = classCC; lookUp = namedLookup   };
    let testAcc    : Accessor = { kind = #element; form = #test;    fourcc = classCC; lookUp = testLookup    };
    // Collection-only:
    //   'pcnt' — `count of <collection>`.
    //   'prop' — `<propName> of every <elem>`: maps the requested
    //            property across matched elements via `findAccessor`
    //            on each child Smurf (no per-T schema baked in).
    let pcntAcc : Accessor = {
      kind   = #element;   // synthetic: backs `count`, not a real AEOM property → excluded from pALL
      form   = #named;
      fourcc = "pcnt";
      lookUp = func _ = simpleLeaf(#int32 (intToInt32Wrap (cardinality())));
    };
    let propAcc : Accessor = {
      kind   = #element;   // synthetic: `<prop> of every <elem>` projector → excluded from pALL
      form   = #named;
      fourcc = "prop";
      lookUp = func(par : Smurf, key : LookupKey) : Smurf =
        switch key {
          case (#named propName) {
            // Project propName across each matching element and wrap
            // the resulting [Smurf] in a smurfMap — Functor lift, so
            // subsequent navigation (e.g. `nth char of every name`)
            // distributes through.
            let count = cardinality();
            let resBuf = Array_init<Smurf>(count, notFoundSmurf par);
            var present : Nat = 0;
            iteri<T>(source, func(srcIdx, t) {
              if (passes t) {
                let elem = wrap(t, srcIdx, parent);
                switch (findAccessor(elem, propName, #named)) {
                  case (?acc) { resBuf[present] := acc.lookUp(elem, #named propName); present += 1 };
                  case null ();
                };
              };
            });
            let results = Array_tabulate<Smurf>(present, func j = resBuf[j]);
            smurfMap(par, results)
          };
          case _ notFoundSmurf par;
        };
    };
    public let accessors : [Accessor] = switch (parentHasIndexed, parentHasNamed, parentHasTest) {
      case (true,  true,  true)  [indexedAcc, namedAcc, testAcc, pcntAcc, propAcc];
      case (true,  true,  false) [indexedAcc, namedAcc, pcntAcc, propAcc];
      case (true,  false, true)  [indexedAcc, testAcc, pcntAcc, propAcc];
      case (true,  false, false) [indexedAcc, pcntAcc, propAcc];
      case (false, true,  true)  [namedAcc, testAcc, pcntAcc, propAcc];
      case (false, true,  false) [namedAcc, pcntAcc, propAcc];
      case (false, false, true)  [testAcc, pcntAcc, propAcc];
      case (false, false, false) [pcntAcc, propAcc];
    };
    // Block-body avoids the M0137 outer-scope leak (#6133).
    public func toDesc() : async* ObjectSpec {
      let count = cardinality();
      let buf = Array_init<ObjectSpec>(count, #root);
      var i : Nat = 0;
      var srcIdx : Nat = 0;
      for (t in source.vals()) {
        srcIdx += 1;
        if (passes t) { buf[i] := await* wrap(t, srcIdx, parent).toDesc(); i += 1 };
      };
      #list (Array_tabulate<ObjectSpec>(count, func j = buf[j]))
    };
    public func filter(p : BoolExpr) : Smurf {
      let newPred : ?BoolExpr = switch pred {
        case null ?p;
        case (?old) ?(#and_ (old, p));
      };
      CollectionSmurf<T>(source, classCC, parent, wrap, lookup, getName, newPred)
    };
  };

  // FlattenedSmurf<P, E>: 1→many join.  Given a parent `[P]` and an
  // `extract : P -> [E]`, eagerly materialise the flat `[E]` and
  // present it as a CollectionSmurf<E>.  Same surface as
  // CollectionSmurf<E> — the class instance is structurally a
  // CollectionSmurf<E> via field re-export.  Eager-only for now;
  // streaming variant can come if N grows large enough to matter.
  public class FlattenedSmurf<P, E>(
    parents  : [P],
    extract  : P -> [E],
    classCC  : Text,
    parent   : Smurf,
    wrap    : (E, Nat, Smurf) -> Smurf,   // Nat = 1-based slot in flattened source
    lookup  : Text -> (E -> CandidValue),
    getName : E -> Text,
    pred     : ?BoolExpr,
  ) : CollectionSmurf<E> {
    // Materialise per-parent extractions, then flatten by index
    // decomposition (no default-E required — Array_tabulate's body
    // computes each output position from the perParent slices).
    let perParent = Array_tabulate<[E]>(parents.size(), func i = extract(parents[i]));
    var totalN : Nat = 0;
    for (es in perParent.vals()) totalN += es.size();
    let flat : [E] = Array_tabulate<E>(totalN, func k {
      var rem = k;
      var pi : Nat = 0;
      while (rem >= perParent[pi].size()) {
        rem -= perParent[pi].size();
        pi += 1;
      };
      perParent[pi][rem]
    });

    let inner : Smurf = CollectionSmurf<E>(flat, classCC, parent, wrap, lookup, getName, pred);

    public let {class4cc; accessors} = inner;
    public func toDesc() : async* ObjectSpec { await* inner.toDesc() };
    public func filter(p : BoolExpr) : Smurf { inner.filter p };
  };

  // ── AE decoder ────────────────────────────────────────────────────────────

  public class Reader(src : Iter<Nat8>) {
    public let next = src.next;

    public func take(n : Nat) : ?Blob {
      let raw = Array_init<Nat8>(n, 0);
      for (i in raw.keys()) {
        let ?b = next() else return null;
        raw[i] := b;
      };
      ?arrayMutToBlob(raw);
    };

    public func readU32() : ?Nat32 = do ? {
      func to32(b : Nat8, s : Nat32) : Nat32 = intToNat32Wrap(nat8ToNat(b)) << s;
      to32(next()!, 24) | to32(next()!, 16) | to32(next()!, 8) | to32(next()!, 0);
    };
  };

  // 4cc descriptor types
  let (DLE2, OBJ, NULL)          = (0x646c6532 : Nat32, 0x6f626a20 : Nat32, 0x6e756c6c : Nat32);
  // 4cc obj record keys
  let (WANT, FORM, SELD, FROM)   = (0x77616e74 : Nat32, 0x666f726d : Nat32, 0x73656c64 : Nat32, 0x66726f6d : Nat32);
  // 4cc payload tags
  let (TYPE, ENUM, PROP, TEST, NAME) = (0x74797065 : Nat32, 0x656e756d : Nat32, 0x70726f70 : Nat32, 0x74657374 : Nat32, 0x6e616d65 : Nat32);
  // primitive value descriptors + 'exmn'
  let (UTXT, LONG, EXMN)         = (0x75747874 : Nat32, 0x6c6f6e67 : Nat32, 0x65786d6e : Nat32);
  // AE boolean literals
  let (AE_TRUE, AE_FALSE)        = (0x74727520 : Nat32, 0x66616c73 : Nat32);
  // formAbsolutePosition
  let (INDX, ABSO)               = (0x696e6478 : Nat32, 0x6162736f : Nat32);
  let ID : Nat32                 = 0x49442020;  // formUniqueID 'ID  '
  let RECO : Nat32               = 0x7265636f;  // typeAERecord 'reco'
  let (AE_ALL, AE_FIRST, AE_LAST, AE_ANY, AE_MIDD) =
    (0x616c6c20 : Nat32, 0x66697273 : Nat32, 0x6c617374 : Nat32, 0x616e7920 : Nat32, 0x6d696464 : Nat32);
  // predicate descriptors
  let (LOGI, CMPD, LIST)         = (0x6c6f6769 : Nat32, 0x636d7064 : Nat32, 0x6c697374 : Nat32);
  // 'logi'/'cmpd' record keys
  let (LOGC, TERM, OBJ1, RELO, OBJ2) =
    (0x6c6f6763 : Nat32, 0x7465726d : Nat32, 0x6f626a31 : Nat32, 0x72656c6f : Nat32, 0x6f626a32 : Nat32);
  // 'logc' enum values
  let (AND_OP, OR_OP, NOT_OP)    = (0x414e4420 : Nat32, 0x4f522020 : Nat32, 0x4e4f5420 : Nat32);
  // 'relo' enum values (AE has no !=; #ne emitted as NOT(=))
  let (EQ_OP, LT_OP, GT_OP, LE_OP, GE_OP) =
    (0x3d202020 : Nat32, 0x3c202020 : Nat32, 0x3e202020 : Nat32, 0x3c3d2020 : Nat32, 0x3e3d2020 : Nat32);
  // kAEContains ('cont'): relo for `{set} contains {item}`
  let CONT_OP : Nat32 = 0x636f6e74;

  func u32(r : Reader) : Nat32 {
    let ?n = r.readU32() else trap "AE: short read";
    n
  };

  func cc4ToText(cc : Nat32) : Text {
    let b = Array_init<Nat8>(4, 0);
    b[0] := natToNat8(nat32ToNat((cc >> 24) & 0xff));
    b[1] := natToNat8(nat32ToNat((cc >> 16) & 0xff));
    b[2] := natToNat8(nat32ToNat((cc >> 8) & 0xff));
    b[3] := natToNat8(nat32ToNat(cc & 0xff));
    let ?t = decodeUtf8(arrayMutToBlob b) else trap "AE: invalid utf8 in 4cc";
    t
  };

  func parseDescBody(typeCode : Nat32, r : Reader) : ObjectSpec {
    let length = u32 r;
    if (typeCode == NULL or typeCode == EXMN) {
      if (length != 0) trap "AE: null/exmn desc with non-zero length";
      #root
    } else if (typeCode == OBJ) {
      parseObjBody r
    } else {
      trap "AE: unsupported ObjectSpec descriptor type"
    }
  };

  func parseObjBody(r : Reader) : ObjectSpec {
    let _fieldCount = u32 r;
    let _padding = u32 r;
    var class_ : Text = "";
    var formCode : Nat32 = PROP;
    var key : KeyForm = #property "";
    var container : ObjectSpec = #root;
    var i = 0;
    while (i < 4) {
      let keyCode = u32 r;
      let valueType = u32 r;
      if (keyCode == FROM) {
        container := parseDescBody(valueType, r);
      } else if (keyCode == WANT) {
        let _len = u32 r;
        class_ := cc4ToText(u32 r);
      } else if (keyCode == FORM) {
        let _len = u32 r;
        formCode := u32 r;
      } else if (keyCode == SELD) {
        if (formCode == PROP) {
          let _len = u32 r;
          key := #property (cc4ToText(u32 r));
        } else if (formCode == TEST) {
          key := #test (parseBoolExprBody(valueType, r));
        } else if (formCode == NAME) {
          if (valueType != UTXT) trap ("AE: form=name expects utxt seld, got " # cc4ToText valueType);
          let bodyLen = u32 r;
          let ?body = r.take(nat32ToNat bodyLen) else trap "AE: short utxt body in form=name seld";
          key := #name (utxtToText body);
        } else if (formCode == INDX) {
          if (valueType == ABSO) {
            let _len = u32 r;
            let enumVal = u32 r;
            key :=
              if      (enumVal == AE_ALL)  #every
              else if (enumVal == AE_FIRST) #absolutePosition 1
              else if (enumVal == AE_LAST)  #absolutePosition (-1)
              else if (enumVal == AE_ANY)   #absolutePosition 1   // FUDGE: AE 'any ' → pos 1
              else if (enumVal == AE_MIDD)  #absolutePosition (-1) // FUDGE: AE 'midd' → last
              else trap ("AE: unsupported 'abso' enum " # cc4ToText enumVal);
          } else if (valueType == LONG) {
            let _len = u32 r;
            key := #absolutePosition (int32ToInt (nat32ToInt32 (u32 r)));
          } else trap ("AE: unsupported 'indx' seld type " # cc4ToText valueType);
        } else if (formCode == ID) {
          // formUniqueID payload, decoded straight into a CandidValue:
          //  • scalar  — a single object by its key text (`order id "ORD0000"`) → #text;
          //  • AERecord `{left:…, right:…, on:…}` (the keyed join spec)          → #record;
          //  • AEList   `{a, b}` (positional join spec; empty `{}` = 0 items)    → #list.
          if (valueType == RECO) {
            let _len = u32 r;
            let count = u32 r;
            let _pad  = u32 r;
            let nf = nat32ToNat count;
            let fields = Array_init<(Text, CandidValue)>(nf, ("", #null_));
            for (j in fields.keys()) {
              let kw = cc4ToText (u32 r);
              fields[j] := (kw, parseValue r);
            };
            key := #id (#record (Array_tabulate<(Text, CandidValue)>(nf, func j = fields[j])));
          } else if (valueType == LIST) {
            let _len = u32 r;
            key := #id (#list (parseInListBody r));   // count + pad + items
          } else if (valueType == UTXT) {
            let bodyLen = u32 r;
            let ?body = r.take(nat32ToNat bodyLen) else trap "AE: short utxt id seld";
            key := #id (#text (utxtToText body));
          } else {
            let len = u32 r;
            let ?_body = r.take(nat32ToNat len) else trap "AE: short seld body in form=id";
            key := #id (#null_);
          };
        } else trap ("AE: unsupported form code " # cc4ToText formCode);
      } else {
        trap "AE: unknown obj field key"
      };
      i += 1;
    };
    #obj { class_; container; key }
  };

  public func parseTopLevel(r : Reader) : ObjectSpec {
    if (u32 r != DLE2) trap "AE: missing dle2 magic";
    if (u32 r != 0)    trap "AE: dle2 padding nonzero";
    parseDescBody(u32 r, r)
  };

  func utxtToText(body : Blob) : Text {
    if (body.size() % 2 != 0) trap "AE: utxt body odd length";
    var out : Text = "";
    var hi : Nat = 0;
    var i : Nat = 0;
    for (b in body.vals()) {
      if (i % 2 == 0) { hi := nat8ToNat b }
      else {
        let cp = hi * 256 + nat8ToNat b;
        if (cp >= 0xD800 and cp <= 0xDFFF) trap "AE: surrogate pair not supported in utxt";
        out #= charToText(nat32ToChar(natToNat32 cp));
      };
      i += 1;
    };
    out
  };

  func parseValueBody(typeCode : Nat32, length : Nat32, r : Reader) : CandidValue {
    if (typeCode == NULL) {
      if (length != 0) trap "AE: null value with non-zero length";
      #null_
    } else if (typeCode == UTXT) {
      let ?body = r.take(nat32ToNat length) else trap "AE: short utxt body";
      #text (utxtToText body)
    } else if (typeCode == LONG) {
      if (length != 4) trap "AE: long must be 4 bytes";
      #int32 (nat32ToInt32 (u32 r))
    } else if (typeCode == ENUM) {
      if (length != 4) trap "AE: enum must be 4 bytes";
      #text (cc4ToText (u32 r))
    } else if (typeCode == AE_TRUE) {
      if (length != 0) trap "AE: 'tru ' value with non-zero length";
      #bool true
    } else if (typeCode == AE_FALSE) {
      if (length != 0) trap "AE: 'fals' value with non-zero length";
      #bool false
    } else if (typeCode == TYPE) {
      if (length != 4) trap "AE: type (class constant) must be 4 bytes";
      #type_ (cc4ToText (u32 r))
    } else {
      trap "AE: unsupported value type"
    }
  };

  func parseValue(r : Reader) : CandidValue = parseValueBody(u32 r, u32 r, r);

  func parseDescFromBody(typeCode : Nat32, r : Reader) : ObjectSpec {
    if (typeCode == NULL or typeCode == EXMN) #root
    else if (typeCode == OBJ) parseObjBody r
    else trap ("AE: unsupported ObjectSpec descriptor type " # cc4ToText typeCode)
  };

  // Apple embeds lists in cmpd fields as count(4)+pad(4)+items (no sub-header).
  func parseInListBody(r : Reader) : [CandidValue] {
    let count = u32 r;
    let _pad  = u32 r;
    let n = nat32ToNat count;
    let vals = Array_init<CandidValue>(n, #null_);
    for (j in vals.keys()) { vals[j] := parseValue r };
    Array_tabulate<CandidValue>(n, func j = vals[j])
  };

  func parseBoolExprBody(typeCode : Nat32, r : Reader) : BoolExpr {
    let _length = u32 r;
    if (typeCode == LOGI) parseLogiBody r
    else if (typeCode == CMPD) parseCmpdBody r
    else if (typeCode == OBJ) {
      // Terse `whose <bool-prop>` form — interpret as `<prop> = true`.
      let spec = parseObjBody r;
      switch spec {
        case (#obj { class_ = _; container = _; key = #property p })
          #compare { prop = p; op = #eq; value = #bool true };
        case _ trap "AE: BoolExpr obj-spec must be a property reference (form=prop)";
      }
    }
    else trap "AE: unsupported BoolExpr descriptor"
  };

  func parseBoolExpr(r : Reader) : BoolExpr {
    let typeCode = u32 r;
    parseBoolExprBody(typeCode, r)
  };

  func parseLogiBody(r : Reader) : BoolExpr {
    let _fc  = u32 r;
    let _pad = u32 r;
    let logcKey = u32 r;
    if (logcKey != LOGC) trap "AE: expected 'logc'";
    let _logcType = u32 r;
    let _logcLen  = u32 r;
    let op = u32 r;
    let termKey = u32 r;
    if (termKey != TERM) trap "AE: expected 'term'";
    let _termType = u32 r;
    let _termLen  = u32 r;
    let count = u32 r;
    let _listPad = u32 r;
    let n = nat32ToNat count;
    if (n == 0) trap "AE: empty term list";
    var acc = parseBoolExpr r;
    var i : Nat = 1;
    while (i < n) {
      let next = parseBoolExpr r;
      if      (op == AND_OP) acc := #and_ (acc, next)
      else if (op == OR_OP)  acc := #or_  (acc, next)
      else trap "AE: unsupported logical op for fold";
      i += 1;
    };
    if (op == NOT_OP) #not_ acc else acc
  };

  func parseCmpdBody(r : Reader) : BoolExpr {
    let _fc  = u32 r;
    let _pad = u32 r;
    var obj1T : Nat32 = 0; var obj1B : ?Blob = null;
    var reloB : ?Blob = null;
    var obj2T : Nat32 = 0; var obj2N : Nat32 = 0; var obj2B : ?Blob = null;
    var i = 0;
    while (i < 3) {
      let key = u32 r;
      let t = u32 r;
      let n = u32 r;
      let ?body = r.take(nat32ToNat n) else trap "AE: cmpd field truncated";
      if      (key == OBJ1) { obj1T := t; obj1B := ?body }
      else if (key == RELO) { reloB := ?body }
      else if (key == OBJ2) { obj2T := t; obj2N := n; obj2B := ?body }
      else trap ("AE: unknown cmpd field key " # debug_show key);
      i += 1;
    };
    let ?rb = reloB else trap "AE: cmpd missing relo";
    let ?opCode = Reader(rb.vals()).readU32() else trap "AE: cmpd relo too short";
    let ?b1 = obj1B else trap "AE: cmpd missing obj1";
    let ?b2 = obj2B else trap "AE: cmpd missing obj2";
    if (opCode == CONT_OP) {
      if (obj1T != LIST) trap ("AE: #contains obj1 must be typeAEList, got " # cc4ToText obj1T);
      let values = parseInListBody(Reader(b1.vals()));
      let prop = switch (parseDescFromBody(obj2T, Reader(b2.vals()))) {
        case (#obj { key = #property p; container = _; class_ = _ }) p;
        case _ trap "AE: cmpd 'cont' obj2 is not a property reference";
      };
      #contains { prop; values }
    } else {
      let op = if      (opCode == EQ_OP) #eq
               else if (opCode == LT_OP) #lt
               else if (opCode == GT_OP) #gt
               else if (opCode == LE_OP) #le
               else if (opCode == GE_OP) #ge
               else trap "AE: unsupported relo opcode";
      let prop = switch (parseDescFromBody(obj1T, Reader(b1.vals()))) {
        case (#obj { key = #property p; container = _; class_ = _ }) p;
        case _ trap "AE: cmpd 'obj1' is not a property reference";
      };
      let val = parseValueBody(obj2T, obj2N, Reader(b2.vals()));
      #compare { prop; op; value = val }
    }
  };

  // ── AE encoder ────────────────────────────────────────────────────────────

  public class Writer(size : Nat) {
    let buf = Array_init<Nat8>(size, 0);
    var pos = 0;

    public func writeU32(n : Nat32) {
      buf[pos]     := natToNat8(nat32ToNat((n >> 24) & 0xff));
      buf[pos + 1] := natToNat8(nat32ToNat((n >> 16) & 0xff));
      buf[pos + 2] := natToNat8(nat32ToNat((n >> 8) & 0xff));
      buf[pos + 3] := natToNat8(nat32ToNat(n & 0xff));
      pos += 4;
    };

    public func writeU32s(ns : [Nat32]) { for (n in ns.vals()) writeU32 n };
    public func writeBytes(b : Blob) { for (byte in b.vals()) { buf[pos] := byte; pos += 1 } };
    public func toBlob() : Blob = arrayMutToBlob buf;
  };

  func textToCC4(t : Text) : Nat32 {
    let blob = encodeUtf8 t;
    if (blob.size() > 4) trap "AE: 4cc text exceeds 4 bytes";
    let pad = Array_init<Nat8>(4, 0);
    var i = 0;
    for (b in blob.vals()) { pad[i] := b; i += 1 };
    let ?n = Reader((arrayMutToBlob pad).vals()).readU32() else trap "AE: short cc4";
    n
  };

  func valueDescLen(v : CandidValue) : Nat {
    switch v {
      case (#null_)   0;
      case (#text t)  2 * utf16Units t;
      case (#int32 _) 4;
      case (#bool _)  0;
      case (#type_ _) 4;
      case _ trap "AE: encoder unsupported value type";
    }
  };

  func boolExprDescLen(e : BoolExpr) : Nat {
    switch e {
      case (#and_ (a, b))                          60 + boolExprDescLen a + boolExprDescLen b;
      case (#or_  (a, b))                          60 + boolExprDescLen a + boolExprDescLen b;
      case (#not_ a)                               52 + boolExprDescLen a;
      case (#always)                               trap "AE: #always is OSL-internal, not wire-encodable";
      case (#contains _)                           trap "AE: #contains is input-only, not wire-encodable";
      case (#compare { prop = _; op = _; value })  116 + valueDescLen value;
    }
  };

  func seldBodyLen(key : KeyForm) : Nat {
    switch key {
      case (#property _)         4;
      case (#name n)             2 * utf16Units n;
      case (#test e)             boolExprDescLen e;
      case (#every)              4;
      case (#absolutePosition _) 4;
      case _ trap "AE: encoder unsupported key form";
    }
  };

  func encDescLen(spec : ObjectSpec) : Nat {
    switch spec {
      case (#root)                              0;
      case (#obj { class_ = _; container; key }) 64 + seldBodyLen key + encDescLen container;
      case (#value v)                            valueDescLen v;
      case (#list es) {
        var n : Nat = 24;  // 8 align + 8 sub-header + 8 count+pad
        for (e in es.vals()) n += 8 + encDescLen e;
        n
      };
    }
  };

  func textToUtf16(t : Text) : Blob {
    let n = utf16Units t;
    let buf = Array_init<Nat8>(n * 2, 0);
    var i = 0;
    for (c in t.chars()) {
      let cp = nat32ToNat (charToNat32 c);
      if (cp > 0xFFFF) trap "AE: non-BMP codepoint not yet supported in utxt";
      buf[i * 2]     := natToNat8(cp / 256);
      buf[i * 2 + 1] := natToNat8(cp % 256);
      i += 1;
    };
    arrayMutToBlob buf
  };

  func utf16Units(t : Text) : Nat {
    var n : Nat = 0;
    for (c in t.chars()) {
      if (nat32ToNat (charToNat32 c) > 0xFFFF) trap "AE: non-BMP codepoint not yet supported in utxt";
      n += 1;
    };
    n
  };

  func compareOpCC(op : Comparison) : Nat32 {
    switch op {
      case (#eq) EQ_OP; case (#lt) LT_OP; case (#gt) GT_OP;
      case (#le) LE_OP; case (#ge) GE_OP;
      case (#ne) trap "AE: #ne should be NOT(=) — TODO";
    }
  };

  func writeValue(w : Writer, v : CandidValue) {
    switch v {
      case (#null_)   w.writeU32s([NULL, 0]);
      case (#text t)  { let b = textToUtf16 t; w.writeU32s([UTXT, intToNat32Wrap (b.size())]); w.writeBytes b };
      case (#int32 i) w.writeU32s([LONG, 4, int32ToNat32 i]);
      case (#bool b)  w.writeU32s([if b AE_TRUE else AE_FALSE, 0]);
      case (#type_ cc) w.writeU32s([TYPE, 4, textToCC4 cc]);
      case _ trap "AE: encoder unsupported value type";
    }
  };

  func writeLogiHeader(w : Writer, op : Nat32, count : Nat32, listBodyLen : Nat) {
    w.writeU32s([2, 0, LOGC, ENUM, 4, op, TERM, LIST, intToNat32Wrap listBodyLen, count, 0]);
  };

  func writeBoolExpr(w : Writer, e : BoolExpr) {
    let len = boolExprDescLen e;
    switch e {
      case (#and_ (a, b)) {
        w.writeU32s([LOGI, intToNat32Wrap len]);
        writeLogiHeader(w, AND_OP, 2, 24 + boolExprDescLen a + boolExprDescLen b);
        writeBoolExpr(w, a); writeBoolExpr(w, b);
      };
      case (#or_ (a, b)) {
        w.writeU32s([LOGI, intToNat32Wrap len]);
        writeLogiHeader(w, OR_OP, 2, 24 + boolExprDescLen a + boolExprDescLen b);
        writeBoolExpr(w, a); writeBoolExpr(w, b);
      };
      case (#not_ a) {
        w.writeU32s([LOGI, intToNat32Wrap len]);
        writeLogiHeader(w, NOT_OP, 1, 16 + boolExprDescLen a);
        writeBoolExpr(w, a);
      };
      case (#compare { prop; op; value }) {
        w.writeU32s([CMPD, intToNat32Wrap len, 3, 0, OBJ1]);
        writeDesc(w, #obj { class_ = "prop"; container = #root; key = #property prop });
        w.writeU32s([RELO, ENUM, 4, compareOpCC op, OBJ2]);
        writeValue(w, value);
      };
      case (#always _)    trap "AE: #always is OSL-internal, not wire-encodable";
      case (#contains _)  trap "AE: #contains is input-only, not wire-encodable";
    }
  };

  func writeObjBody(w : Writer, class_ : Text, container : ObjectSpec, key : KeyForm) {
    let formCode = switch key {
      case (#property _)          PROP;
      case (#name _)              NAME;
      case (#test _)              TEST;
      case (#every)               INDX;
      case (#absolutePosition _)  INDX;
      case _ trap "AE: encoder key form unsupported";
    };
    w.writeU32s([4, 0, WANT, TYPE, 4, textToCC4 class_, FORM, ENUM, 4, formCode, SELD]);
    switch key {
      case (#property name)          w.writeU32s([TYPE, 4, textToCC4 name]);
      case (#name n)                 { let b = textToUtf16 n; w.writeU32s([UTXT, intToNat32Wrap (b.size())]); w.writeBytes b };
      case (#test e)                 writeBoolExpr(w, e);
      case (#every)                  w.writeU32s([ABSO, 4, AE_ALL]);
      case (#absolutePosition i)     w.writeU32s([LONG, 4, int32ToNat32 (intToInt32Wrap i)]);
      case _ trap "AE: encoder unsupported key form";
    };
    w.writeU32 FROM;
    writeDesc(w, container);
  };

  func writeDesc(w : Writer, spec : ObjectSpec) {
    switch spec {
      case (#root)                       w.writeU32s([NULL, 0]);
      case (#obj { class_; container; key }) {
        w.writeU32s([OBJ, intToNat32Wrap (encDescLen spec)]);
        writeObjBody(w, class_, container, key);
      };
      case (#value v) writeValue(w, v);
      case (#list es) {
        w.writeU32s([LIST, intToNat32Wrap (encDescLen spec),
                     0, 0, 0x18, LIST, intToNat32Wrap (es.size()), 0]);
        for (e in es.vals()) writeDesc(w, e);
      };
    }
  };

  public func encodeAE(spec : ObjectSpec) : Blob {
    let w = Writer(16 + encDescLen spec);
    w.writeU32s([DLE2, 0]);
    writeDesc(w, spec);
    w.toBlob()
  };

  // ── Spec walker ───────────────────────────────────────────────────────────

  func formOfKey(k : KeyForm) : { #indexed; #named; #test; #id } =
    switch k {
      case (#absolutePosition _) #indexed;
      case (#name _)             #named;
      case (#uniqueID _)         #named;
      case (#property _)         #named;
      case (#range _)            #indexed;
      case (#test _)             #test;
      case (#every)              #test;
      case (#id _)               #id;
    };

  func lookupOfKey(k : KeyForm) : ?LookupKey =
    switch k {
      case (#absolutePosition i) ?(#indexed i);
      case (#name n)             ?(#named n);
      case (#property p)         ?(#named p);
      case (#test e)             ?(#test e);
      case (#every)              ?(#test (#always));
      case (#id ids)             ?(#id ids);
      case (#uniqueID _)         null;
      case (#range _)            null;
    };

  func resolve(spec : ObjectSpec, root : Smurf) : Smurf {
    switch spec {
      case (#root) root;
      case (#value _ or #list _) trap "AE: non-navigable spec at navigation position";
      case (#obj { class_; container; key }) {
        let parent = resolve(container, root);
        let form = formOfKey key;
        let ?lk = lookupOfKey key else trap "AE: unsupported keyform (uniqueID/range)";
        let acc_opt = switch (findAccessor(parent, class_, form), class_, key) {
          case (?a, _, _)                       ?a;
          case (null, "prop", #property "pALL") ?pAllAccessor;
          case (null, "prop", #property p)      findAccessor(parent, p, form);
          case _                                null;
        };
        let ?acc = acc_opt else return notFoundSmurf parent;
        acc.lookUp(parent, lk)
      };
    }
  };

  public func eval(spec : ObjectSpec, root : Smurf) : async* ObjectSpec {
    await* resolve(spec, root).toDesc()
  };

  // ── Lingo types ───────────────────────────────────────────────────────────

  public type LingoValueType = { #Text; #Integer; #Real; #Boolean; #ClassRef : Text; #Any };
  public type LingoAccess    = { #readOnly; #writeOnly; #readWrite };

  public type LingoProperty = {
    name       : Text;
    code       : Text;
    valueType  : LingoValueType;
    access     : LingoAccess;
    description : ?Text;
  };

  public type LingoElement = { classCode : Text; access : LingoAccess };

  public type LingoClass = {
    name        : Text;
    code        : Text;
    plural      : Text;
    description : ?Text;
    properties  : [LingoProperty];
    elements    : [LingoElement];
  };

  public type Lingo = {
    suiteName   : Text;
    suiteCode   : Text;
    description : ?Text;
    classes     : [LingoClass];
  };

}
