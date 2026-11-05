# cost_vs_ref: wasmtime fuel per operation

Harness: wasmtime 42.0.1; wasmtime::Config::new() with wasm_memory64(true) and consume_fuel(true), every other setting default (Cranelift, OptLevel::Speed, default per-operator fuel costs); the fuel of a call is Store::get_fuel() before it minus after it, the store refilled to 2^60 whenever it falls below 2^50; 200 samples per row.

Module: nt-cost for wasm64-unknown-unknown, rustc 1.99.0-nightly (da86f4d07 2026-07-24), -Z build-std=std,panic_abort, release profile (opt-level 3, panic abort, codegen-units 1); source e20f046.

Mean fuel per call over the samples, the export's own instructions included (an empty export costs 2); the iteration rows give fuel per element. `ref` is the transcription of today's PBTreeMap, `unc` the crate's typed tree with CHECKED false, `chk` with CHECKED true. Δ is against `ref`; a negative Δ means cheaper. "Equal" counts the samples whose fuel equals `ref`'s exactly.

Gate on `unc`: get and insert with room at or below `ref` (mean at most +0.05 fuel, no single sample more than 1 above); every other row at most max(4 fuel, 0.5% of `ref`) above `ref` on average. Result: **pass**.

| shape | n | depth | operation | samples | ref | unc | Δ unc | equal | limit | gate | chk | Δ chk |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---:|
| <u64, u64> | 1000 | 2 | get (hit) | 200 | 381.36 | 381.36 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 459.98 | +78.62 (+20.62%) |
| <u64, u64> | 1000 | 2 | insert, room | 200 | 651.22 | 651.22 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 781.80 | +130.58 (+20.05%) |
| <u64, u64> | 1000 | 2 | insert, split | 200 | 5841.46 | 5842.50 | +1.04 (+0.02%) | 1/200 | ≤ +29.21 | pass | 6150.52 | +309.06 (+5.29%) |
| <u64, u64> | 1000 | 2 | entry, vacant (one search, insert along the saved path) | 200 | 683.64 | 683.66 | +0.02 (+0.00%) | 196/200 | ≤ +4.00 | pass | 811.64 | +128.00 (+18.72%) |
| <u64, u64> | 1000 | 2 | remove, leaf | 200 | 576.46 | 570.51 | -5.95 (-1.03%) | 0/200 | ≤ +4.00 | pass | 706.59 | +130.13 (+22.57%) |
| <u64, u64> | 1000 | 2 | remove, merge at the leaf level | 200 | 831.98 | 819.56 | -12.42 (-1.49%) | 0/200 | ≤ +4.16 | pass | 1241.44 | +409.47 (+49.22%) |
| <u64, u64> | 1000 | 2 | remove, rotation from the left at the leaf level | 200 | 734.37 | 729.37 | -5.00 (-0.68%) | 0/200 | ≤ +4.00 | pass | 1067.01 | +332.64 (+45.30%) |
| <u64, u64> | 1000 | 2 | remove, rotation from the right at the leaf level | 200 | 540.00 | 536.00 | -4.00 (-0.74%) | 0/200 | ≤ +4.00 | pass | 853.00 | +313.00 (+57.96%) |
| <u64, u64> | 1000 | 2 | remove, leaf merge, then merge at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, u64> | 1000 | 2 | remove, leaf merge, then rotation from the left at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, u64> | 1000 | 2 | remove, leaf merge, then rotation from the right at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, u64> | 1000 | 2 | seek and first step, start unbounded | 200 | 226.00 | 209.00 | -17.00 (-7.52%) | 0/200 | ≤ +4.00 | pass | 220.00 | -6.00 (-2.65%) |
| <u64, u64> | 1000 | 2 | seek and first step, start included | 200 | 568.10 | 564.10 | -4.00 (-0.70%) | 0/200 | ≤ +4.00 | pass | 669.47 | +101.38 (+17.84%) |
| <u64, u64> | 1000 | 2 | seek and first step, start excluded | 200 | 580.40 | 576.88 | -3.52 (-0.61%) | 0/200 | ≤ +4.00 | pass | 680.32 | +99.92 (+17.21%) |
| <u64, u64> | 1000 | 2 | next, end unbounded | 5120 | 99.78 | 94.70 | -5.08 (-5.09%) | 787/5120 | ≤ +4.00 | pass | 104.14 | +4.36 (+4.37%) |
| <u64, u64> | 1000 | 2 | next, end included | 5120 | 110.87 | 105.31 | -5.56 (-5.01%) | 383/5120 | ≤ +4.00 | pass | 115.64 | +4.77 (+4.31%) |
| <u64, u64> | 1000 | 2 | next, end excluded | 5120 | 116.37 | 110.45 | -5.93 (-5.09%) | 74/5120 | ≤ +4.00 | pass | 121.43 | +5.06 (+4.35%) |
| <u64, u64> | 100000 | 3 | get (hit) | 200 | 645.01 | 645.01 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 774.15 | +129.14 (+20.02%) |
| <u64, u64> | 100000 | 3 | insert, room | 200 | 932.65 | 932.65 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 1112.41 | +179.76 (+19.27%) |
| <u64, u64> | 100000 | 3 | insert, split | 200 | 6149.05 | 6149.38 | +0.32 (+0.01%) | 2/200 | ≤ +30.75 | pass | 6525.34 | +376.29 (+6.12%) |
| <u64, u64> | 100000 | 3 | entry, vacant (one search, insert along the saved path) | 200 | 958.28 | 958.29 | +0.01 (+0.00%) | 198/200 | ≤ +4.79 | pass | 1135.73 | +177.45 (+18.52%) |
| <u64, u64> | 100000 | 3 | remove, leaf | 200 | 885.08 | 879.12 | -5.95 (-0.67%) | 0/200 | ≤ +4.43 | pass | 1054.86 | +169.78 (+19.18%) |
| <u64, u64> | 100000 | 3 | remove, merge at the leaf level | 200 | 1152.64 | 1142.69 | -9.95 (-0.86%) | 0/200 | ≤ +5.76 | pass | 1614.46 | +461.82 (+40.07%) |
| <u64, u64> | 100000 | 3 | remove, rotation from the left at the leaf level | 200 | 1036.86 | 1031.88 | -4.97 (-0.48%) | 1/200 | ≤ +5.18 | pass | 1409.11 | +372.25 (+35.90%) |
| <u64, u64> | 100000 | 3 | remove, rotation from the right at the leaf level | 200 | 843.97 | 839.97 | -4.00 (-0.47%) | 0/200 | ≤ +4.22 | pass | 1197.73 | +353.76 (+41.92%) |
| <u64, u64> | 100000 | 3 | remove, leaf merge, then merge at an internal level | 200 | 1224.74 | 1208.74 | -16.00 (-1.31%) | 0/200 | ≤ +6.12 | pass | 1958.12 | +733.38 (+59.88%) |
| <u64, u64> | 100000 | 3 | remove, leaf merge, then rotation from the left at an internal level | 200 | 1145.88 | 1136.88 | -9.00 (-0.79%) | 0/200 | ≤ +5.73 | pass | 1805.16 | +659.29 (+57.54%) |
| <u64, u64> | 100000 | 3 | remove, leaf merge, then rotation from the right at an internal level | 200 | 1139.16 | 1131.16 | -8.00 (-0.70%) | 0/200 | ≤ +5.70 | pass | 1796.87 | +657.70 (+57.74%) |
| <u64, u64> | 100000 | 3 | seek and first step, start unbounded | 200 | 260.00 | 247.00 | -13.00 (-5.00%) | 0/200 | ≤ +4.00 | pass | 258.00 | -2.00 (-0.77%) |
| <u64, u64> | 100000 | 3 | seek and first step, start included | 200 | 852.76 | 847.75 | -5.01 (-0.59%) | 0/200 | ≤ +4.26 | pass | 1012.46 | +159.69 (+18.73%) |
| <u64, u64> | 100000 | 3 | seek and first step, start excluded | 200 | 865.77 | 861.26 | -4.51 (-0.52%) | 0/200 | ≤ +4.33 | pass | 1024.64 | +158.87 (+18.35%) |
| <u64, u64> | 100000 | 3 | next, end unbounded | 5120 | 110.49 | 104.47 | -6.01 (-5.44%) | 0/5120 | ≤ +4.00 | pass | 115.62 | +5.13 (+4.64%) |
| <u64, u64> | 100000 | 3 | next, end included | 5120 | 116.43 | 110.42 | -6.01 (-5.16%) | 0/5120 | ≤ +4.00 | pass | 121.54 | +5.12 (+4.39%) |
| <u64, u64> | 100000 | 3 | next, end excluded | 5120 | 117.42 | 111.40 | -6.01 (-5.12%) | 0/5120 | ≤ +4.00 | pass | 122.53 | +5.11 (+4.35%) |
| <u64, u64> | 100000 | 3 | iteration, full scan (per element) | 200 | 109.44 | 105.42 | -4.01 (-3.67%) | 0/200 | ≤ +4.00 | pass | 116.55 | +7.12 (+6.50%) |
| <u64, u64> | 100000 | 3 | iteration, bounded range of about 1,000 (per element) | 200 | 118.61 | 112.60 | -6.01 (-5.07%) | 0/200 | ≤ +4.00 | pass | 123.77 | +5.15 (+4.35%) |
| <u64, u64> | 1000000 | 3 | get (hit) | 200 | 760.90 | 760.90 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 903.45 | +142.54 (+18.73%) |
| <u64, u64> | 1000000 | 3 | insert, room | 200 | 1049.63 | 1049.65 | +0.02 (+0.00%) | 197/200 | = on every sample | pass | 1242.67 | +193.04 (+18.39%) |
| <u64, u64> | 1000000 | 3 | insert, split | 200 | 6378.95 | 6379.26 | +0.31 (+0.00%) | 6/200 | ≤ +31.89 | pass | 6772.11 | +393.16 (+6.16%) |
| <u64, u64> | 1000000 | 3 | entry, vacant (one search, insert along the saved path) | 200 | 1079.38 | 1079.39 | +0.01 (+0.00%) | 199/200 | ≤ +5.40 | pass | 1270.83 | +191.44 (+17.74%) |
| <u64, u64> | 1000000 | 3 | remove, leaf | 200 | 1010.04 | 1004.07 | -5.97 (-0.59%) | 0/200 | ≤ +5.05 | pass | 1190.63 | +180.59 (+17.88%) |
| <u64, u64> | 1000000 | 3 | remove, merge at the leaf level | 200 | 1271.39 | 1261.39 | -10.00 (-0.79%) | 0/200 | ≤ +6.36 | pass | 1744.82 | +473.43 (+37.24%) |
| <u64, u64> | 1000000 | 3 | remove, rotation from the left at the leaf level | 200 | 1141.55 | 1136.57 | -4.98 (-0.44%) | 1/200 | ≤ +5.71 | pass | 1522.57 | +381.02 (+33.38%) |
| <u64, u64> | 1000000 | 3 | remove, rotation from the right at the leaf level | 200 | 948.12 | 944.12 | -4.00 (-0.42%) | 0/200 | ≤ +4.74 | pass | 1310.32 | +362.19 (+38.20%) |
| <u64, u64> | 1000000 | 3 | remove, leaf merge, then merge at an internal level | 200 | 1366.97 | 1350.97 | -16.00 (-1.17%) | 0/200 | ≤ +6.83 | pass | 2144.49 | +777.52 (+56.88%) |
| <u64, u64> | 1000000 | 3 | remove, leaf merge, then rotation from the left at an internal level | 200 | 1261.40 | 1252.40 | -9.00 (-0.71%) | 0/200 | ≤ +6.31 | pass | 1928.65 | +667.25 (+52.90%) |
| <u64, u64> | 1000000 | 3 | remove, leaf merge, then rotation from the right at an internal level | 200 | 1238.62 | 1230.62 | -8.00 (-0.65%) | 0/200 | ≤ +6.19 | pass | 1905.90 | +667.29 (+53.87%) |
| <u64, u64> | 1000000 | 3 | seek and first step, start unbounded | 200 | 260.00 | 247.00 | -13.00 (-5.00%) | 0/200 | ≤ +4.00 | pass | 258.00 | -2.00 (-0.77%) |
| <u64, u64> | 1000000 | 3 | seek and first step, start included | 200 | 962.07 | 957.07 | -5.00 (-0.52%) | 0/200 | ≤ +4.81 | pass | 1137.87 | +175.80 (+18.27%) |
| <u64, u64> | 1000000 | 3 | seek and first step, start excluded | 200 | 968.67 | 964.16 | -4.51 (-0.47%) | 0/200 | ≤ +4.84 | pass | 1142.42 | +173.75 (+17.94%) |
| <u64, u64> | 1000000 | 3 | next, end unbounded | 5120 | 110.39 | 104.38 | -6.01 (-5.45%) | 0/5120 | ≤ +4.00 | pass | 115.49 | +5.10 (+4.62%) |
| <u64, u64> | 1000000 | 3 | next, end included | 5120 | 116.41 | 110.40 | -6.01 (-5.16%) | 0/5120 | ≤ +4.00 | pass | 121.52 | +5.11 (+4.39%) |
| <u64, u64> | 1000000 | 3 | next, end excluded | 5120 | 117.47 | 111.46 | -6.01 (-5.12%) | 0/5120 | ≤ +4.00 | pass | 122.59 | +5.12 (+4.36%) |
| <u64, u64> | 1000000 | 3 | iteration, full scan (per element) | 200 | 109.40 | 105.39 | -4.01 (-3.67%) | 0/200 | ≤ +4.00 | pass | 116.50 | +7.11 (+6.50%) |
| <u64, u64> | 1000000 | 3 | iteration, bounded range of about 1,000 (per element) | 200 | 119.00 | 112.99 | -6.01 (-5.05%) | 0/200 | ≤ +4.00 | pass | 124.23 | +5.23 (+4.39%) |
| <u64, [u8; 1024]> | 10000 | 2 | get (hit) | 200 | 490.36 | 490.36 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 581.24 | +90.88 (+18.53%) |
| <u64, [u8; 1024]> | 10000 | 2 | insert, room | 200 | 860.92 | 860.93 | +0.00 (+0.00%) | 199/200 | = on every sample | pass | 1004.51 | +143.59 (+16.68%) |
| <u64, [u8; 1024]> | 10000 | 2 | insert, split | 200 | 6032.73 | 6032.14 | -0.60 (-0.01%) | 1/200 | ≤ +30.16 | pass | 6372.68 | +339.94 (+5.63%) |
| <u64, [u8; 1024]> | 10000 | 2 | entry, vacant (one search, insert along the saved path) | 200 | 863.32 | 863.33 | +0.01 (+0.00%) | 197/200 | ≤ +4.32 | pass | 1004.69 | +141.38 (+16.38%) |
| <u64, [u8; 1024]> | 10000 | 2 | remove, leaf | 200 | 748.77 | 742.80 | -5.98 (-0.80%) | 0/200 | ≤ +4.00 | pass | 899.11 | +150.34 (+20.08%) |
| <u64, [u8; 1024]> | 10000 | 2 | remove, merge at the leaf level | 200 | 992.88 | 982.84 | -10.04 (-1.01%) | 0/200 | ≤ +4.96 | pass | 1441.47 | +448.60 (+45.18%) |
| <u64, [u8; 1024]> | 10000 | 2 | remove, rotation from the left at the leaf level | 200 | 895.32 | 890.37 | -4.95 (-0.55%) | 2/200 | ≤ +4.48 | pass | 1246.75 | +351.43 (+39.25%) |
| <u64, [u8; 1024]> | 10000 | 2 | remove, rotation from the right at the leaf level | 200 | 712.00 | 709.00 | -3.00 (-0.42%) | 0/200 | ≤ +4.00 | pass | 1047.00 | +335.00 (+47.05%) |
| <u64, [u8; 1024]> | 10000 | 2 | remove, leaf merge, then merge at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, [u8; 1024]> | 10000 | 2 | remove, leaf merge, then rotation from the left at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, [u8; 1024]> | 10000 | 2 | remove, leaf merge, then rotation from the right at an internal level | – | – | – | – | – | – | n/a: no internal level below the root | – | – |
| <u64, [u8; 1024]> | 10000 | 2 | seek and first step, start unbounded | 200 | 227.00 | 210.00 | -17.00 (-7.49%) | 0/200 | ≤ +4.00 | pass | 221.00 | -6.00 (-2.64%) |
| <u64, [u8; 1024]> | 10000 | 2 | seek and first step, start included | 200 | 678.15 | 674.14 | -4.01 (-0.59%) | 0/200 | ≤ +4.00 | pass | 795.42 | +117.27 (+17.29%) |
| <u64, [u8; 1024]> | 10000 | 2 | seek and first step, start excluded | 200 | 687.60 | 684.11 | -3.50 (-0.51%) | 0/200 | ≤ +4.00 | pass | 803.08 | +115.48 (+16.79%) |
| <u64, [u8; 1024]> | 10000 | 2 | next, end unbounded | 5120 | 111.51 | 105.50 | -6.01 (-5.39%) | 0/5120 | ≤ +4.00 | pass | 116.65 | +5.13 (+4.60%) |
| <u64, [u8; 1024]> | 10000 | 2 | next, end included | 5120 | 113.97 | 108.23 | -5.73 (-5.03%) | 236/5120 | ≤ +4.00 | pass | 118.87 | +4.91 (+4.31%) |
| <u64, [u8; 1024]> | 10000 | 2 | next, end excluded | 5120 | 118.26 | 112.27 | -5.99 (-5.07%) | 16/5120 | ≤ +4.00 | pass | 123.38 | +5.13 (+4.33%) |
| <PString, u64> | 100000 | 3 | get (hit) | 200 | 2707.34 | 2707.34 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 2849.91 | +142.57 (+5.27%) |
| <PString, u64> | 100000 | 3 | insert, room | 200 | 2957.82 | 2922.82 | -35.00 (-1.18%) | 0/200 | = on every sample | pass | 3111.72 | +153.90 (+5.20%) |
| <PString, u64> | 100000 | 3 | insert, split | 200 | 8304.05 | 8264.26 | -39.78 (-0.48%) | 0/200 | ≤ +41.52 | pass | 8620.89 | +316.84 (+3.82%) |
| <PString, u64> | 100000 | 3 | entry, vacant (one search, insert along the saved path) | 200 | 3027.49 | 2991.47 | -36.01 (-1.19%) | 0/200 | ≤ +15.14 | pass | 3179.88 | +152.39 (+5.03%) |
| <PString, u64> | 100000 | 3 | remove, leaf | 200 | 3074.50 | 3041.47 | -33.03 (-1.07%) | 0/200 | ≤ +15.37 | pass | 3254.86 | +180.36 (+5.87%) |
| <PString, u64> | 100000 | 3 | remove, merge at the leaf level | 200 | 3408.97 | 3377.97 | -31.01 (-0.91%) | 0/200 | ≤ +17.04 | pass | 3884.76 | +475.79 (+13.96%) |
| <PString, u64> | 100000 | 3 | remove, rotation from the left at the leaf level | 200 | 3203.34 | 3170.30 | -33.04 (-1.03%) | 0/200 | ≤ +16.02 | pass | 3577.28 | +373.94 (+11.67%) |
| <PString, u64> | 100000 | 3 | remove, rotation from the right at the leaf level | 200 | 2356.61 | 2326.61 | -30.00 (-1.27%) | 0/200 | ≤ +11.78 | pass | 2705.43 | +348.82 (+14.80%) |
| <PString, u64> | 100000 | 3 | remove, leaf merge, then merge at an internal level | 200 | 2743.28 | 2712.28 | -31.00 (-1.13%) | 0/200 | ≤ +13.72 | pass | 3504.53 | +761.26 (+27.75%) |
| <PString, u64> | 100000 | 3 | remove, leaf merge, then rotation from the left at an internal level | 200 | 2708.41 | 2677.41 | -31.00 (-1.14%) | 0/200 | ≤ +13.54 | pass | 3362.45 | +654.04 (+24.15%) |
| <PString, u64> | 100000 | 3 | remove, leaf merge, then rotation from the right at an internal level | 200 | 3315.76 | 3287.76 | -28.00 (-0.84%) | 0/200 | ≤ +16.58 | pass | 3971.59 | +655.83 (+19.78%) |
| <PString, u64> | 100000 | 3 | seek and first step, start unbounded | 200 | 287.00 | 268.00 | -19.00 (-6.62%) | 0/200 | ≤ +4.00 | pass | 279.00 | -8.00 (-2.79%) |
| <PString, u64> | 100000 | 3 | seek and first step, start included | 200 | 2939.84 | 2934.32 | -5.51 (-0.19%) | 0/200 | ≤ +14.70 | pass | 3098.30 | +158.47 (+5.39%) |
| <PString, u64> | 100000 | 3 | seek and first step, start excluded | 200 | 2945.77 | 2938.76 | -7.01 (-0.24%) | 0/200 | ≤ +14.73 | pass | 3100.79 | +155.02 (+5.26%) |
| <PString, u64> | 100000 | 3 | next, end unbounded | 5120 | 124.64 | 117.62 | -7.02 (-5.63%) | 0/5120 | ≤ +4.00 | pass | 128.81 | +4.17 (+3.35%) |
| <PString, u64> | 100000 | 3 | next, end included | 5120 | 202.62 | 195.61 | -7.02 (-3.46%) | 0/5120 | ≤ +4.00 | pass | 206.79 | +4.17 (+2.06%) |
| <PString, u64> | 100000 | 3 | next, end excluded | 5120 | 203.69 | 196.67 | -7.02 (-3.45%) | 0/5120 | ≤ +4.00 | pass | 207.87 | +4.18 (+2.05%) |
| <[u8; 1013], u64> | 100000 | 3 | get (hit) | 200 | 24509.00 | 24509.00 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 24638.14 | +129.14 (+0.53%) |
| <[u8; 1013], u64> | 100000 | 3 | insert, room | 200 | 1696.16 | 1696.16 | +0.00 (+0.00%) | 200/200 | = on every sample | pass | 1875.92 | +179.76 (+10.60%) |
| <[u8; 1013], u64> | 100000 | 3 | insert, split | 200 | 6926.42 | 6926.74 | +0.32 (+0.00%) | 2/200 | ≤ +34.63 | pass | 7302.71 | +376.29 (+5.43%) |
| <[u8; 1013], u64> | 100000 | 3 | entry, vacant (one search, insert along the saved path) | 200 | 1686.91 | 1686.92 | +0.01 (+0.00%) | 198/200 | ≤ +8.43 | pass | 1864.36 | +177.45 (+10.52%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, leaf | 200 | 24846.24 | 24840.28 | -5.95 (-0.02%) | 0/200 | ≤ +124.23 | pass | 25015.26 | +169.03 (+0.68%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, merge at the leaf level | 200 | 25076.62 | 25066.67 | -9.95 (-0.04%) | 0/200 | ≤ +125.38 | pass | 25537.95 | +461.33 (+1.84%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, rotation from the left at the leaf level | 200 | 24981.31 | 24976.33 | -4.97 (-0.02%) | 1/200 | ≤ +124.91 | pass | 25352.72 | +371.42 (+1.49%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, rotation from the right at the leaf level | 200 | 24499.21 | 24495.21 | -4.00 (-0.02%) | 0/200 | ≤ +122.50 | pass | 24856.72 | +357.51 (+1.46%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, leaf merge, then merge at an internal level | 200 | 24888.88 | 24872.88 | -16.00 (-0.06%) | 0/200 | ≤ +124.44 | pass | 25627.46 | +738.58 (+2.97%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, leaf merge, then rotation from the left at an internal level | 200 | 24820.66 | 24811.66 | -9.00 (-0.04%) | 0/200 | ≤ +124.10 | pass | 25484.85 | +664.19 (+2.68%) |
| <[u8; 1013], u64> | 100000 | 3 | remove, leaf merge, then rotation from the right at an internal level | 200 | 25080.10 | 25072.10 | -8.00 (-0.03%) | 0/200 | ≤ +125.40 | pass | 25742.32 | +662.22 (+2.64%) |
| <[u8; 1013], u64> | 100000 | 3 | seek and first step, start unbounded | 200 | 259.00 | 241.00 | -18.00 (-6.95%) | 0/200 | ≤ +4.00 | pass | 252.00 | -7.00 (-2.70%) |
| <[u8; 1013], u64> | 100000 | 3 | seek and first step, start included | 200 | 13225.02 | 13214.02 | -11.00 (-0.08%) | 0/200 | ≤ +66.13 | pass | 13378.73 | +153.70 (+1.16%) |
| <[u8; 1013], u64> | 100000 | 3 | seek and first step, start excluded | 200 | 13229.29 | 13218.78 | -10.51 (-0.08%) | 0/200 | ≤ +66.15 | pass | 13382.17 | +152.88 (+1.16%) |
| <[u8; 1013], u64> | 100000 | 3 | next, end unbounded | 5120 | 109.49 | 100.48 | -9.01 (-8.23%) | 0/5120 | ≤ +4.00 | pass | 111.62 | +2.14 (+1.95%) |
| <[u8; 1013], u64> | 100000 | 3 | next, end included | 5120 | 136.43 | 127.42 | -9.01 (-6.60%) | 0/5120 | ≤ +4.00 | pass | 138.55 | +2.12 (+1.55%) |
| <[u8; 1013], u64> | 100000 | 3 | next, end excluded | 5120 | 137.42 | 128.41 | -9.01 (-6.55%) | 0/5120 | ≤ +4.00 | pass | 139.53 | +2.12 (+1.54%) |
