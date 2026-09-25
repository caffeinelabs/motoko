/**
[#mod-AssocList]
= `AssocList` -- Association lists
*/

import List "list";

module {
  /*

Association Lists
==================

Association Lists, a la functional programming, in Motoko.

Implements the same interface as `Trie`, but as a linked-list of key-value pairs.

*/

  // polymorphic association linked lists between keys and values
  public type AssocList<K, V> = List.List<(K, V)>;

  /*
   `find`
   --------
   find the value associated with a given key, or null if absent.
  */
  public func find<K, V>(
    al : AssocList<K,V>,
    k:K,
    k_eq:(K, K)->Bool
  )
    : ?V
  {
    func rec(al:AssocList<K,V>) : ?V {
      label profile_assocList_find_rec : (?V)
        switch al {
          case null { label profile_assocList_find_end_fail : (?V) null }
          case ?((hd_k, hd_v), tl) {
            if k_eq(k, hd_k) {
              label profile_assocList_find_end_success : (?V)
              ?hd_v
            } else {
              rec(tl)
            }
          }
        }
    };
    label profile_assocList_find_begin : (?V)
    rec(al)
  };

  /*
   `replace`
   ---------
   replace the value associated with a given key, or add it, if missing.
   returns old value, or null, if no prior value existed.
  */
  public func replace<K, V>(
    al : AssocList<K,V>,
    k:K,
    k_eq:(K, K)->Bool,
    ov: ?V
  )
    : (AssocList<K,V>, ?V)
  {
    func rec(al:AssocList<K,V>) : (AssocList<K,V>, ?V) {
      switch al {
        case null {
          switch ov {
            case null { (null, null) }
            case ?v { (?((k, v), null), null) }
          }
        }
        case ?((hd_k, hd_v), tl) {
          if k_eq(k, hd_k) {
            // if value is null, remove the key; otherwise, replace key's old value
            // return old value
            switch ov {
              case null { (tl, ?hd_v) }
              case ?v { (?((hd_k, v), tl), ?hd_v) }
            }
          } else {
            let (tl2, old_v) = rec(tl);
            (?((hd_k, hd_v), tl2), old_v)
          }
        }
      }
    };
    rec(al)
  };

  /*
   `diff`
   ---------
   The key-value pairs of the final list consist of those pairs of
   the left list whose keys are not present in the right list; the
   values of the right list are irrelevant.
  */
  public func diff<K, V, W>(
    al1: AssocList<K,V>,
    al2: AssocList<K,W>,
    keq: (K, K)->Bool
  )
    : AssocList<K,V>
  {
    func rec(al1:AssocList<K,V>) : AssocList<K,V> {
      switch al1 {
        case null { null }
        case ?((k, v1), tl) {
          switch find<K, W>(al2, k, keq) {
            case null { rec(tl) }
            case ?v2 { ?((k, v1), rec(tl)) }
          }
        }
      }
    };
    rec(al1)
  };

  /*
   `mapAppend`
   --------
  */
  public func mapAppend<K, V, W, X>(
    al1:AssocList<K,V>,
    al2:AssocList<K,W>,
    vbin:(?V, ?W)->X
  )
    : AssocList<K,X> = label profile_assocList_mapAppend : AssocList<K,X>
    {
      func rec(
        al1:AssocList<K,V>,
        al2:AssocList<K,W>
      ) : AssocList<K,X> = label profile_assocList_mapAppend_rec : AssocList<K,X> {
        switch (al1, al2) {
          case (null, null) { null }
          case (?((k, v), al1_), _) { ?((k, vbin(?v, null)), rec(al1_, al2)) }
          case (null, ?((k, v), al2_)) { ?((k, vbin(null, ?v)), rec(null, al2_)) }
        }
      };
      rec(al1, al2)
    };

  public func disjDisjoint<K, V, W, X>(
    al1:AssocList<K,V>,
    al2:AssocList<K,W>,
    vbin:(?V, ?W)->X
  )
    : AssocList<K,X> = label profile_assocList_disjDisjoint : AssocList<K,X>
    {
      mapAppend<K, V, W, X>(al1, al2, vbin)
    };

  /*
   `disj`
   --------
   This operation generalizes the notion of "set union" to finite maps.
   Produces a "disjunctive image" of the two lists, where the values of
   matching keys are combined with the given binary operator.

   For unmatched key-value pairs, the operator is still applied to
   create the value in the image.  To accomodate these various
   situations, the operator accepts optional values, but is never
   applied to (null, null).

  */
  public func disj<K, V, W, X>(
    al1:AssocList<K,V>,
    al2:AssocList<K,W>,
    keq:(K, K)->Bool,
    vbin:(?V, ?W)->X
  )
    : AssocList<K,X>
  {
    func rec1(al1:AssocList<K,V>) : AssocList<K,X> {
      switch al1 {
        case null {
          func rec2(al2:AssocList<K,W>) : AssocList<K,X> {
            switch al2 {
              case null { null }
              case ?((k, v2), tl) {
                switch find<K, V>(al1, k, keq) {
                  case null { ?((k, vbin(null, ?v2)), rec2(tl)) }
                  case ?v1 { ?((k, vbin(?v1, ?v2)), rec2(tl)) }
                }
              }
            }
          };
          rec2(al2)
        }
        case ?((k, v1), tl) {
          switch find<K, W>(al2, k, keq) {
            case null { ?((k, vbin(?v1, null)), rec1(tl)) }
            case ?v2 { /* handled above */ rec1(tl) }
          }
        }
      }
    };
    rec1(al1)
  };

  /*
   `join`
   ---------
   This operation generalizes the notion of "set intersection" to
   finite maps.  Produces a "conjuctive image" of the two lists, where
   the values of matching keys are combined with the given binary
   operator, and unmatched key-value pairs are not present in the output.

  */
  public func join<K, V, W, X>(
    al1 : AssocList<K,V>,
    al2:AssocList<K,W>,
    keq:(K, K)->Bool,
    vbin:(V, W)->X
  )
    : AssocList<K,X>
  {
    func rec(al1:AssocList<K,V>) : AssocList<K,X> {
      switch al1 {
        case null { null }
        case ?((k, v1), tl) {
          switch find<K, W>(al2, k, keq) {
            case null { rec(tl) }
            case ?v2 { ?((k, vbin(v1, v2)), rec(tl)) }
          }
        }
      }
    };
    rec(al1)
  };

  /*
   `fold`
   ---------
  */
  public func fold<K, V, X>(
    al:AssocList<K,V>,
    nil:X,
    cons:(K, V, X)->X
  )
    : X
  {
    func rec(al:AssocList<K,V>) : X {
      switch al {
        case null { nil }
        case ?((k, v), t) { cons(k, v, rec(t)) }
      }
    };
    rec(al)
  };

}
