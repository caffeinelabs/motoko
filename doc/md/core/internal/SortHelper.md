# core/internal/SortHelper

## Function `insertionSortSmall`
``` motoko no-repl
func insertionSortSmall<T>(buffer : [var T], dest : [var T], compare : (T, T) -> Order.Order, newFrom : Nat32, len : Nat32)
```


## Function `insertionSortSmallMove`
``` motoko no-repl
func insertionSortSmallMove<T>(buffer : [var T], dest : [var T], compare : (T, T) -> Order.Order, newFrom : Nat32, len : Nat32, offset : Nat32)
```

