/// Text values
///
/// This type represents human-readable text as sequences of characters of type [`Char`](Char.html) .
/// If `t` is a value of type `Text`, then:
/// * `t.chars()` returns an _iterator_ of type `Iter<Char>` enumerating its characters from first to last.
/// * `t.size()` returns the _size_ (or length) of `t` (and `t.chars()`) as a `Nat`.
///
/// This module defines additional operations on `Text` values.

import Char "Char";
import Iter "Iter";
import Hash "Hash";
import Prim "mo:⛔";

module {

  /// Conversion.
  /// Returns the text value of size 1 containing the single character `c`.
  public let fromChar : (c : Char) -> Text = Prim.charToText;

  /// Conversion.
  /// Creates an [iterator](Iter.html#type.Iter) that traverses the characters of the text `t`.
  public func toIter(t : Text) : Iter.Iter<Char> =
    t.chars();

  /// Conversion.
  /// Returns the text value containing the sequence of characters in `cs`.
  public func fromIter(cs : Iter.Iter<Char>) : Text {
    var r = "";
    for (c in cs) {
      r #= Prim.charToText(c);
    };
    return r;
  };

  /// Returns `t.size()`, the number of characters in `t` (and `t.chars()`).
  public func size(t : Text) : Nat { t.size(); };

  /// Returns the concatenation of `t1` and `t2`, `t1 # t2`.
  public func concat(t1 : Text, t2 : Text) : Text =
    t1 # t2;

  /// Returns `t1 == t2`.
  public func equal(t1 : Text, t2 : Text) : Bool { t1 == t2 };

  /// Returns `t1 != t2`.
  public func notEqual(t1 : Text, t2 : Text) : Bool { t1 != t2 };

  /// Returns `t1 < t2`.
  public func less(t1 : Text, t2 : Text) : Bool { t1 < t2 };

  /// Returns `t1 <= t2`.
  public func lessOrEqual(t1 : Text, t2 : Text) : Bool { t1 <= t2 };

  /// Returns `t1 > t2`.
  public func greater(t1 : Text, t2 : Text) : Bool { t1 > t2 };

  /// Returns `t1 >= t2`.
  public func greaterOrEqual(t1 : Text, t2 : Text) : Bool { t1 >= t2 };

  /// Returns the order of `t1` and `t1`.
  public func compare(t1 : Text, t2 : Text) : { #less; #equal; #greater } {
    if (t1 < t2) { #less }
    else if (t1 == t2) { #equal }
    else { #greater }
  };


  private func extract(t : Text, i : Nat, j : Nat) : Text {
    let size = t.size();
    if (i == 0 and j == size) return t;
    assert (j <= size);
    let cs = t.chars();
    var r = "";
    var n = i;
    while (n > 0) {
      ignore cs.next();
      n -= 1;
    };
    n := j;
    while (n > 0) {
      switch (cs.next()) {
        case null { assert false };
        case (?c) { r #= Prim.charToText(c) }
      };
      n -= 1;
    };
    return r;
  };

  /// Returns the concatenation of text values in `ts`, separated by `sep`.
  public func join(sep : Text, ts : Iter.Iter<Text>) : Text {
    var r = "";
    if (sep.size() == 0) {
      for (t in ts) {
        r #= t
      };
      return r;
    };
    let next = ts.next;
    switch (next()) {
      case null { return r; };
      case (?t) {
        r #= t;
      }
    };
    loop {
      switch (next()) {
        case null { return r; };
        case (?t) {
          r #= sep;
          r #= t;
        }
      }
    }
  };


  /// Returns the result of applying `f` to each character in `ts`, concatenating the intermediate single-character text values.
  public func map(t : Text, f : Char -> Char) : Text {
    var r = "";
    for (c in t.chars()) {
      r #= Prim.charToText(f(c));
    };
    return r;
  };

  /// Returns the result of applying `f` to each character in `ts`, concatenating the intermediate text values.
  public func translate(t : Text, f : Char -> Text) : Text {
    var r = "";
    for (c in t.chars()) {
      r #= f(c);
    };
    return r;
  };


  /// A pattern `p` describes a sequence of characters. A pattern has one of the following forms:
  ///
  /// * `#char c` matches the single character sequence, `c`.
  /// * `#predicate p` matches any single character sequence `c` satisfying predicate `p(c)`.
  /// * `#text t` matches multi-character text sequence `t`.
  ///
  /// A _match_ for `p` is any sequence of characters matching the pattern `p`.
  public type Pattern = { #char : Char; #text : Text; #predicate : (Char -> Bool) };

  private func take(n : Nat, cs : Iter.Iter<Char>) : Iter.Iter<Char> {
    var i = n;
    object {
      public func next() : ?Char {
        if (i == 0) return null;
        i -= 1;
        return cs.next();
      }
    }
  };

  private func empty() : Iter.Iter<Char> {
    object {
      public func next() : ?Char = null;
    };
  };

  private type Match = {
    /// #success on complete match
    #success;
    /// #fail(cs,c) on partial match of cs, but failing match on c
    #fail : (cs: Iter.Iter<Char>, c : Char);
    /// #empty(cs) on partial match of cs and empty stream
    #empty : (cs :Iter.Iter<Char> )
  };

  private func sizeOfPattern(pat : Pattern) : Nat {
    switch pat {
      case (#text(t)) { t.size() };
      case (#predicate(_) or #char(_)) { 1 };
    }
  };

  private func matchOfPattern(pat : Pattern) : (cs : Iter.Iter<Char>) -> Match {
     switch pat {
       case (#char(p)) {
         func (cs : Iter.Iter<Char>) : Match {
           switch (cs.next()) {
             case (?c) { 
               if (p == c) { 
                 #success 
               } else { 
                 #fail (empty(), c) } 
               };
             case null { #empty(empty()) };
           }
         }
       };
       case (#predicate(p)) {
         func (cs : Iter.Iter<Char>) : Match {
           switch (cs.next()) {
             case (?c) { 
               if (p(c)) { 
                 #success 
               } else { 
                 #fail(empty(), c) } 
               };
             case null { #empty (empty()) };
           }
         }
       };
       case (#text(p)) {
         func (cs : Iter.Iter<Char>) : Match {
           var i = 0;
           let ds = p.chars();
           loop {
             switch (ds.next()) {
               case (?d)  {
                 switch (cs.next()) {
                   case (?c) {
                     if (c != d) {
                       return #fail (take(i, p.chars()), c)
                     };
                     i += 1;
                   };
                   case null {
                     return #empty (take(i, p.chars()));
                   }
                 }
               };
               case null { return #success };
             }
           }
         }
       }
     }
  };

  private class CharBuffer(cs : Iter.Iter<Char>) : Iter.Iter<Char> = {

    var buff : Iter.Iter<Char> = empty();
    var char : ?Char = null;

    public func pushBack(cs0: Iter.Iter<Char>, c : Char) {
       buff := cs0;
       char := ?c;
    };

    public func next() : ?Char {
      switch (buff.next()) {
        case null {
          switch char {
            case (?c) {
              char := null;
              return ?c;
            };
            case null {
              return cs.next();
            };
          }
        };
        case oc { oc };
      }
    };
  };

  /// Returns the sequence of fields in `t`, derived from start to end,
  /// separated by text matching [pattern](#type.Pattern) `p`.
  /// Two fields are separated by exactly one match.
  public func split(t : Text, p : Pattern) : Iter.Iter<Text> {
    let match = matchOfPattern(p);
    let cs = CharBuffer(t.chars());
    var state = 0;
    var field = "";
    object {
      public func next() : ?Text {
        switch state {
          case (0 or 1) {
            loop {
              switch (match(cs)) {
                case (#success) {
                  let r = field;
                  field := "";
                  state := 1;
                  return ?r
                };
                case (#empty(cs1)) {
                  for (c in cs1) {
                    field #= fromChar(c);
                  };
                  let r =
                    if (state == 0 and field == "") {
                      null
                    } else {
                      ?field
                    };
                  state := 2;
                  return r;
                };
                case (#fail (cs1, c)) {
                  cs.pushBack(cs1,c);
                  switch (cs.next()) {
                    case (?ci) {
                      field #= fromChar(ci);
                    };
                    case null {
                      let r =
                         if (state == 0 and field == "") {
                           null
                         } else {
                           ?field
                         };
                      state := 2;
                      return r;
                    }
                  }
                }
              }
            }
          };
          case _ { return null };
        }
      }
    }
  };

  /// Returns the sequence of tokens in `t`, derived from start to end.
  /// A _token_ is a non-empty maximal subsequence of `t` not containing a match for [pattern](#type.Pattern) `p`.
  /// Two tokens may be separated by one or more matches of `p`.
  public func tokens(t : Text, p : Pattern) : Iter.Iter<Text> {
    let fs = split(t, p);
    object {
      public func next() : ?Text {
        switch (fs.next()) {
          case (?"") { next() };
          case ot { ot };
        }
      }
    }
  };

  /// Returns true if `t` contains a match for [pattern](#type.Pattern) `p`.
  public func contains(t : Text, p : Pattern) : Bool {
    let match = matchOfPattern(p);
    let cs = CharBuffer(t.chars());
    loop {
      switch (match(cs)) {
        case (#success) {
          return true
        };
        case (#empty(cs1)) {
          return false;
        };
        case (#fail(cs1, c)) {
          cs.pushBack(cs1, c);
          switch (cs.next()) {
            case null {
              return false
            };
            case _ { }; // continue
          }
        }
      }
    }
  };

  /// Returns `true` if `t` starts with a prefix matching [pattern](#type.Pattern) `p`, otherwise returns `false`.
  public func startsWith(t : Text, p : Pattern) : Bool {
    var cs = t.chars();
    let match = matchOfPattern(p);
    switch (match(cs)) {
      case (#success) { true };
      case _ { false };
    }
  };

  /// Returns `true` if `t` ends with a suffix matching [pattern](#type.Pattern) `p`, otherwise returns `false`.
  public func endsWith(t : Text, p : Pattern) : Bool {
    let s2 = sizeOfPattern(p);
    if (s2 == 0) return true;
    let s1 = t.size();
    if (s2 > s1) return false;
    let match = matchOfPattern(p);
    var cs1 = t.chars();
    var diff : Nat = s1 - s2;
    while (diff > 0)  {
      ignore cs1.next();
      diff -= 1;
    };
    switch (match(cs1)) {
      case (#success) { true };
      case _ { false };
    }
  };

  /// Returns `t` with all matches of [pattern](#type.Pattern) `p` replaced by text `r`.
  public func replace(t : Text, p : Pattern, r : Text) : Text {
    let match = matchOfPattern(p);
    let size = sizeOfPattern(p);
    let cs = CharBuffer(t.chars());
    var res = "";
    loop {
      switch (match(cs)) {
        case (#success) {
          res #= r;
          if (size > 0) {
            continue;
          }
        };
        case (#empty(cs1)) {
          for (c1 in cs1) {
            res #= fromChar(c1);
          };
          break;
        };
        case (#fail (cs1, c)) {
          cs.pushBack(cs1, c);
        }
      };
      switch (cs.next()) {
        case null {
          break;
        };
        case (?c1) {
         res #= fromChar(c1);
        }; // continue
      }
    };
    return res;
  };



  /// Returns the optioned suffix of `t` obtained by eliding exactly one leading match of [pattern](#type.Pattern) `p`, otherwise `null`.
  public func stripStart(t : Text, p : Pattern) : ?Text {
    let s = sizeOfPattern(p);
    if (s == 0) return ?t;
    var cs = t.chars();
    let match = matchOfPattern(p);
    switch (match(cs)) {
      case (#success) return ?fromIter(cs);
      case _ return null;
    }
  };

  /// Returns the optioned prefix of `t` obtained by eliding exactly one trailing match of [pattern](#type.Pattern) `p`, otherwise `null`.
  public func stripEnd(t : Text, p : Pattern) : ?Text {
    let s2 = sizeOfPattern(p);
    if (s2 == 0) return ?t;
    let s1 = t.size();
    if (s2 > s1) return null;
    let match = matchOfPattern(p);
    var cs1 = t.chars();
    var diff : Nat = s1 - s2;
    while (diff > 0) {
      ignore cs1.next();
      diff -= 1;
    };
    switch (match(cs1)) {
      case (#success) return ?extract(t, 0, s1 - s2);
      case _ return null;
    }
  };

  /// Returns the suffix of `t` obtained by eliding all leading matches of [pattern](#type.Pattern) `p`.
  public func trimStart(t : Text, p : Pattern) : Text {
    let cs = t.chars();
    let size = sizeOfPattern(p);
    if (size == 0) return t;
    var matchSize = 0;
    let match = matchOfPattern(p);
    loop {
      switch (match(cs)) {
        case (#success) {
          matchSize += size;
        }; // continue
        case (#empty(cs1)) {
          return if (matchSize == 0) { 
            t 
          } else {
            fromIter(cs1)
          } 
        };
        case (#fail (cs1, c)) {
          return if (matchSize == 0) {
            t
          } else {
            fromIter(cs1) # fromChar(c) # fromIter(cs)
          }
        }
      }
    }
  };

  /// Returns the prefix of `t` obtained by eliding all trailing matches of [pattern](#type.Pattern) `p`.
  public func trimEnd(t : Text, p : Pattern) : Text {
    let cs = CharBuffer(t.chars());
    let size = sizeOfPattern(p);
    if (size == 0) return t;
    let match = matchOfPattern(p);
    var matchSize = 0;
    label l
    loop {
      switch (match(cs)) {
        case (#success) {
          matchSize += size;
        }; // continue
        case (#empty(cs1)) {
          switch (cs1.next()) {
            case null break l;
            case (?_) return t;
          }
        };
        case (#fail (cs1, c)) {
          matchSize := 0;
          cs.pushBack(cs1, c);
          ignore cs.next();
        }
      }
    };
    extract(t, 0, t.size() - matchSize)
  };

  /// Returns the subtext of `t` obtained by eliding all leading and trailing matches of [pattern](#type.Pattern) `p`.
  public func trim(t : Text, p : Pattern) : Text {
    let cs = t.chars();
    let size = sizeOfPattern(p);
    if (size == 0) return t;
    var matchSize = 0;
    let match = matchOfPattern(p);
    loop {
      switch (match(cs)) {
        case (#success) {
          matchSize += size;
        }; // continue
        case (#empty(cs1)) {
          return if (matchSize == 0) { t } else { fromIter(cs1) }
        };
        case (#fail (cs1, c)) {
          let start = matchSize;
          let cs2 = CharBuffer(cs);
          cs2.pushBack(cs1, c);
          ignore cs2.next();
          matchSize := 0;
          label l
          loop {
            switch (match(cs2)) {
              case (#success) {
                matchSize += size;
              }; // continue
              case (#empty(cs3)) {
                switch (cs1.next()) {
                  case null break l;
                  case (?_) return t;
                }
              };
              case (#fail (cs3, c1)) {
                matchSize := 0;
                cs2.pushBack(cs3, c1);
                ignore cs2.next();
              }
            }
          };
          return extract(t, start, t.size() - matchSize - start);
        }
      }
    }
  };

  /// Returns the lexicographic comparison of `t1` and `t2`, using the given character ordering `cmp`.
  public func compareWith(
    t1 : Text,
    t2 : Text,
    cmp : (Char, Char)-> { #less; #equal; #greater })
    : { #less; #equal; #greater } {
    let cs1 = t1.chars();
    let cs2 = t2.chars();
    loop {
      switch (cs1.next(), cs2.next()) {
        case (null, null) { return #equal };
        case (null, ?_) { return #less };
        case (?_, null) { return #greater };
        case (?c1, ?c2) {
          switch (cmp(c1, c2)) {
            case (#equal) { }; // continue
            case other { return other; }
          }
        }
      }
    }
  };

}
