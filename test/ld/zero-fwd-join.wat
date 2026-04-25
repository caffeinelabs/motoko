(module
  (type $t_bi (func (param i32) (param i32) (result i32)))
  (type $t_i  (func (param i32) (result i32)))

  (import "rts" "quux" (func $quux (type $t_bi)))

  (table (;0;) i64 1 1 funcref)
  (memory (;0;) i64 2)
  (global $heap_base i64 (i64.const 65536))
  (export "__heap_base" (global $heap_base))
  (export "nested_block"     (func $nested_block))
  (export "block_over_loop"  (func $block_over_loop))
  (export "agreeing_brif"    (func $agreeing_brif))
  (export "agreeing_if_legs" (func $agreeing_if_legs))
  (export "br_table_agree"   (func $br_table_agree))

  ;; Zero-forwarder: body = Const 0; LocalGet 1; Call $quux — matches
  ;; zero_forwarder_target's syntactic pattern, so the linker records
  ;; $bar -> $quux in zero_fwds. Rewrites at any call site where
  ;; ConstTrack reports `I32 0` at depth n_params-1.
  (func $bar (type $t_bi)
    i32.const 0
    local.get 1
    call $quux)

  ;; Variant with a trailing `Return`. moc occasionally emits one
  ;; after a tail call; zero_forwarder_target accepts both shapes
  ;; (`[Call k]` and `[Call k; Return]`).
  (func $bar_ret (type $t_bi)
    i32.const 0
    local.get 1
    call $quux
    return)

  ;; Case 1 — "Nested-Block de-Bruijn depth" (reviewer ex. 1)
  ;; Br 1 from an inner block carries `i32.const 0` to the outer
  ;; block's End. A second Br 1 from the fall-through path also
  ;; carries 0. Phase 3's handler decrements the depth through the
  ;; inner handler so the outer Block's branch_states collects both.
  ;; Phase 2 had no accumulation — its conservative None-from-
  ;; terminator path left the outer LRU empty.
  (func $nested_block (type $t_i)
    (block (result i32)
      (block
        i32.const 0
        local.get 0
        br_if 1         ;; taken: outer-End with 0
        br 1            ;; fall-through: also exits outer-End with 0
      )
      unreachable
    )
    local.get 0
    call $bar)

  ;; Case 2 — "Block { Loop { Br 1 } }" (reviewer ex. 2)
  ;; Loop doesn't collect — depth-0 back-edges are swallowed — but
  ;; the Loop handler still decrements n>0 depths so the outer Block
  ;; catches `Br 1` with its carried state. Phase 2 didn't process
  ;; the loop body at all (old `Loop (bt, _body)` ignored it), so
  ;; the carried 0 was invisible.
  (func $block_over_loop (type $t_i)
    (block (result i32)
      (loop
        i32.const 0
        br 1            ;; exits outer with 0
      )
      unreachable
    )
    local.get 0
    call $bar)

  ;; Case 3 — "Agreeing BrIf" (reviewer ex. 3, canonical precision
  ;; test). The Block has a BrIf; taken and fall-through paths both
  ;; produce `i32.const 0`. Phase 2 evicted result-slot entries on
  ;; any saw_br_if sighting, so the 0 was lost. Phase 3 intersects
  ;; the two agreeing states and preserves it.
  ;;
  ;; This case also exercises the `[Call k; Return]` arm of
  ;; zero_forwarder_target by calling $bar_ret (trailing-Return
  ;; variant) instead of $bar.
  (func $agreeing_brif (type $t_i)
    (block (result i32)
      i32.const 0
      local.get 0
      br_if 0
      drop
      i32.const 0
    )
    local.get 0
    call $bar_ret)

  ;; Case 4 — "If with agreeing legs" (reviewer ex. 4). Both then
  ;; and else produce `i32.const 0`; the then-leg contains a BrIf
  ;; targeting the function label. Phase 2's shared saw_br_if across
  ;; the two legs forced result-slot eviction even though both legs
  ;; agreed. Phase 3 intersects the two legs' fall-through states
  ;; (normalised via the unified branch_states trick) and keeps the 0.
  (func $agreeing_if_legs (type $t_i)
    local.get 0
    (if (result i32)
      (then
        i32.const 0
        local.get 0
        br_if 1         ;; exit function with 0
        drop
        i32.const 0)
      (else
        i32.const 0))
    local.get 0
    call $bar)

  ;; Case 5 — BrTable (grande finale)
  ;; All listed targets and the default point at the enclosing Block's
  ;; End, and the popped-index state carries `i32.const 0` on top.
  ;; ConstTrack emits one `May_leave (0, [d0=I32 0])` per target; the
  ;; Block's branch_states collects them all and intersects to the same
  ;; 0. The previous `BrTable _ -> None` arm gave the analyser nothing,
  ;; so the rewrite didn't fire.
  (func $br_table_agree (type $t_i)
    (block (result i32)
      i32.const 0
      local.get 0
      br_table 0 0 0 0        ;; three listed + default, all target this block
    )
    local.get 0
    call $bar))
