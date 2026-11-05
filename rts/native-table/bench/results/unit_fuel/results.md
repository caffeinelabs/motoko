
Units cut by their own estimates only (instructions on the replica, by its cost model):

| scenario | job and step | units | most instructions | mean instructions | most estimated | most changed pages | most charged | longest between meter readings |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| mixed rows: seal, compaction of deletes and patches, a drop | compaction begin | 1 | 23267 | 23267 | 20992 | 8 | 55267 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction flip | 1 | 21959 | 21959 | 23340 | 4 | 37959 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction install | 5 | 33468 | 29845 | 34680 | 2 | 41468 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction last | 431 | 34654 | 32090 | 30960 | 7 | 47892 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction layout | 3 | 31948 | 29518 | 32920 | 2 | 39948 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction pass1 | 9841 | 35405 | 26463 | 43196 | 3 | 43405 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction snapshot | 1 | 21289 | 21289 | 25360 | 4 | 37289 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction write | 12984 | 47948 | 36895 | 37480 | 5 | 63221 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | drain | 1219 | 28568 | 15228 | 31300 | 7 | 48630 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal begin | 2 | 20501 | 16534 | 20592 | 8 | 52501 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal flip | 2 | 16377 | 12910 | 16720 | 6 | 40377 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal install | 24 | 29870 | 21907 | 39480 | 3 | 41870 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal last | 77 | 29972 | 18654 | 38650 | 7 | 57972 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal layout | 5 | 34493 | 25277 | 32920 | 3 | 44489 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal pass1 | 1490 | 44958 | 24487 | 38994 | 4 | 60958 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal snapshot | 58 | 19629 | 17755 | 32250 | 4 | 35562 | - |
| mixed rows: seal, compaction of deletes and patches, a drop | seal write | 1092 | 57550 | 32703 | 42830 | 5 | 76904 | - |
| 64 KiB-class values: seal, compaction of patches | compaction begin | 1 | 22046 | 22046 | 20992 | 8 | 54046 | - |
| 64 KiB-class values: seal, compaction of patches | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| 64 KiB-class values: seal, compaction of patches | compaction install | 2 | 26040 | 25088 | 32136 | 3 | 36137 | - |
| 64 KiB-class values: seal, compaction of patches | compaction last | 1 | 22024 | 22024 | 23840 | 5 | 42024 | - |
| 64 KiB-class values: seal, compaction of patches | compaction layout | 1 | 23805 | 23805 | 21640 | 2 | 31805 | - |
| 64 KiB-class values: seal, compaction of patches | compaction pass1 | 249 | 29095 | 25108 | 41208 | 3 | 36829 | - |
| 64 KiB-class values: seal, compaction of patches | compaction snapshot | 1 | 21237 | 21237 | 25360 | 4 | 37237 | - |
| 64 KiB-class values: seal, compaction of patches | compaction write | 1091 | 46055 | 37738 | 35180 | 7 | 74055 | - |
| 64 KiB-class values: seal, compaction of patches | drain | 962 | 26188 | 19669 | 24760 | 7 | 45992 | - |
| 64 KiB-class values: seal, compaction of patches | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 64 KiB-class values: seal, compaction of patches | seal flip | 1 | 8692 | 8692 | 8440 | 5 | 28692 | - |
| 64 KiB-class values: seal, compaction of patches | seal install | 2 | 30632 | 26713 | 32936 | 3 | 38632 | - |
| 64 KiB-class values: seal, compaction of patches | seal last | 1 | 26959 | 26959 | 23840 | 6 | 50959 | - |
| 64 KiB-class values: seal, compaction of patches | seal layout | 1 | 23139 | 23139 | 21640 | 2 | 31139 | - |
| 64 KiB-class values: seal, compaction of patches | seal pass1 | 327 | 36164 | 31207 | 37852 | 2 | 40177 | - |
| 64 KiB-class values: seal, compaction of patches | seal snapshot | 1 | 20388 | 20388 | 24200 | 4 | 36388 | - |
| 64 KiB-class values: seal, compaction of patches | seal write | 900 | 52861 | 46211 | 32020 | 7 | 77602 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction begin | 1 | 19483 | 19483 | 12312 | 5 | 39483 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction flip | 1 | 5252 | 5252 | 8200 | 3 | 17252 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction install | 311 | 32712 | 25617 | 32136 | 6 | 56712 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction last | 78 | 29092 | 27788 | 31240 | 4 | 45092 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction layout | 139 | 51904 | 32765 | 32920 | 3 | 63904 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction pass1 | 147981 | 36177 | 20898 | 35480 | 9 | 68967 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction snapshot | 1 | 19955 | 19955 | 14600 | 5 | 39955 | - |
| 621 leaves: seal, compaction of 600 patch units | compaction write | 77314 | 53270 | 31516 | 35094 | 5 | 65270 | - |
| 621 leaves: seal, compaction of 600 patch units | drain | 253 | 23846 | 17559 | 24920 | 7 | 51621 | - |
| 621 leaves: seal, compaction of 600 patch units | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 621 leaves: seal, compaction of 600 patch units | seal flip | 1 | 6599 | 6599 | 8200 | 5 | 26599 | - |
| 621 leaves: seal, compaction of 600 patch units | seal install | 373 | 34820 | 27476 | 31488 | 6 | 58820 | - |
| 621 leaves: seal, compaction of 600 patch units | seal last | 78 | 29093 | 26921 | 31240 | 5 | 49093 | - |
| 621 leaves: seal, compaction of 600 patch units | seal layout | 78 | 52121 | 51077 | 31240 | 3 | 64121 | - |
| 621 leaves: seal, compaction of 600 patch units | seal pass1 | 9329 | 46960 | 23844 | 38260 | 9 | 82960 | - |
| 621 leaves: seal, compaction of 600 patch units | seal snapshot | 31 | 22965 | 15784 | 34600 | 5 | 38965 | - |
| 621 leaves: seal, compaction of 600 patch units | seal write | 369 | 55826 | 45241 | 32248 | 5 | 75826 | - |
| values sharing long prefixes: seal, compaction | compaction begin | 1 | 22572 | 22572 | 21392 | 8 | 54572 | - |
| values sharing long prefixes: seal, compaction | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| values sharing long prefixes: seal, compaction | compaction install | 3 | 30569 | 27315 | 32136 | 3 | 38569 | - |
| values sharing long prefixes: seal, compaction | compaction last | 11 | 22795 | 18978 | 30940 | 5 | 33003 | - |
| values sharing long prefixes: seal, compaction | compaction layout | 2 | 30078 | 28399 | 32120 | 3 | 38721 | - |
| values sharing long prefixes: seal, compaction | compaction pass1 | 6789 | 41628 | 27629 | 38555 | 4 | 51340 | - |
| values sharing long prefixes: seal, compaction | compaction snapshot | 1 | 21261 | 21261 | 25360 | 4 | 37261 | - |
| values sharing long prefixes: seal, compaction | compaction write | 831 | 46822 | 36850 | 35480 | 6 | 67338 | - |
| values sharing long prefixes: seal, compaction | drain | 696 | 29136 | 20147 | 28760 | 7 | 49136 | - |
| values sharing long prefixes: seal, compaction | seal begin | 1 | 13692 | 13692 | 11912 | 7 | 41692 | - |
| values sharing long prefixes: seal, compaction | seal flip | 1 | 9066 | 9066 | 8600 | 5 | 29066 | - |
| values sharing long prefixes: seal, compaction | seal install | 3 | 34890 | 32738 | 32936 | 4 | 48694 | - |
| values sharing long prefixes: seal, compaction | seal last | 22 | 36215 | 31625 | 31220 | 6 | 43503 | - |
| values sharing long prefixes: seal, compaction | seal layout | 2 | 29472 | 19945 | 32120 | 3 | 41472 | - |
| values sharing long prefixes: seal, compaction | seal pass1 | 10573 | 43962 | 29950 | 37852 | 4 | 59962 | - |
| values sharing long prefixes: seal, compaction | seal snapshot | 2 | 17412 | 16256 | 29200 | 4 | 33412 | - |
| values sharing long prefixes: seal, compaction | seal write | 992 | 52582 | 46186 | 32020 | 7 | 80582 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction begin | 1 | 22908 | 22908 | 17712 | 8 | 54908 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction flip | 1 | 5141 | 5141 | 8200 | 3 | 17141 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction install | 512 | 36697 | 26041 | 32040 | 8 | 60801 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction last | 129 | 28556 | 27792 | 31240 | 5 | 40851 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction layout | 231 | 43978 | 33343 | 35480 | 3 | 55978 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction pass1 | 105534 | 40700 | 22941 | 35960 | 9 | 71717 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction snapshot | 1 | 40092 | 40092 | 21000 | 10 | 80092 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction write | 179751 | 53874 | 28278 | 35495 | 6 | 76942 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | drain | 496 | 29289 | 18620 | 27760 | 8 | 49289 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | drop | 1 | 25702 | 25702 | 17400 | 6 | 49702 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal begin | 2 | 16733 | 15195 | 17312 | 8 | 48733 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal flip | 2 | 10336 | 8436 | 13600 | 8 | 42336 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal install | 1218 | 37846 | 28190 | 31960 | 9 | 67675 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal last | 258 | 28402 | 27279 | 31240 | 6 | 43503 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal layout | 450 | 43409 | 34207 | 35480 | 6 | 55409 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal pass1 | 32169 | 45448 | 24712 | 39144 | 10 | 73448 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal snapshot | 7 | 47674 | 26769 | 36500 | 9 | 83674 | - |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal write | 157510 | 54665 | 22905 | 42768 | 6 | 73313 | - |
| rows of three schema versions: seal, column release, patches | drain | 408 | 25998 | 20396 | 27260 | 7 | 50415 | - |
| rows of three schema versions: seal, column release, patches | release begin | 112 | 33689 | 23090 | 35912 | 5 | 53689 | - |
| rows of three schema versions: seal, column release, patches | release flip | 1 | 5577 | 5577 | 8200 | 3 | 17577 | - |
| rows of three schema versions: seal, column release, patches | release install | 282 | 33301 | 26005 | 32136 | 6 | 57301 | - |
| rows of three schema versions: seal, column release, patches | release last | 71 | 30072 | 28184 | 32040 | 4 | 45488 | - |
| rows of three schema versions: seal, column release, patches | release layout | 142 | 32342 | 30570 | 33320 | 3 | 44342 | - |
| rows of three schema versions: seal, column release, patches | release pass1 | 67102 | 39608 | 29468 | 38384 | 7 | 65468 | - |
| rows of three schema versions: seal, column release, patches | release snapshot | 1 | 23904 | 23904 | 19300 | 8 | 55904 | - |
| rows of three schema versions: seal, column release, patches | release write | 51148 | 56977 | 39470 | 34292 | 4 | 65888 | - |
| rows of three schema versions: seal, column release, patches | seal begin | 1 | 15128 | 15128 | 11912 | 7 | 43128 | - |
| rows of three schema versions: seal, column release, patches | seal flip | 1 | 7562 | 7562 | 8200 | 5 | 27562 | - |
| rows of three schema versions: seal, column release, patches | seal install | 287 | 40192 | 33106 | 33896 | 6 | 64192 | - |
| rows of three schema versions: seal, column release, patches | seal last | 72 | 29602 | 27697 | 31960 | 5 | 49530 | - |
| rows of three schema versions: seal, column release, patches | seal layout | 144 | 32229 | 31212 | 32920 | 3 | 44229 | - |
| rows of three schema versions: seal, column release, patches | seal pass1 | 46126 | 47632 | 31651 | 36936 | 9 | 75632 | - |
| rows of three schema versions: seal, column release, patches | seal snapshot | 31 | 24068 | 17092 | 34200 | 8 | 54616 | - |
| rows of three schema versions: seal, column release, patches | seal write | 38038 | 58132 | 41726 | 42580 | 5 | 74629 | - |
| writes between units, clear in a compaction and in a seal | compaction begin | 1 | 17789 | 17789 | 17712 | 7 | 45789 | - |
| writes between units, clear in a compaction and in a seal | compaction flip | 1 | 24591 | 24591 | 28560 | 4 | 40591 | - |
| writes between units, clear in a compaction and in a seal | compaction install | 5 | 33426 | 27359 | 34680 | 2 | 41426 | - |
| writes between units, clear in a compaction and in a seal | compaction last | 431 | 38073 | 32711 | 31160 | 7 | 45052 | - |
| writes between units, clear in a compaction and in a seal | compaction layout | 3 | 32002 | 28939 | 32920 | 2 | 40002 | - |
| writes between units, clear in a compaction and in a seal | compaction pass1 | 14082 | 35709 | 24414 | 41224 | 5 | 47700 | - |
| writes between units, clear in a compaction and in a seal | compaction snapshot | 1 | 13782 | 13782 | 20000 | 6 | 37782 | - |
| writes between units, clear in a compaction and in a seal | compaction write | 13924 | 47919 | 36232 | 37480 | 6 | 71322 | - |
| writes between units, clear in a compaction and in a seal | drain | 1504 | 28821 | 20000 | 30680 | 7 | 48630 | - |
| writes between units, clear in a compaction and in a seal | seal begin | 2 | 13278 | 12923 | 11912 | 5 | 33278 | - |
| writes between units, clear in a compaction and in a seal | seal flip | 1 | 9444 | 9444 | 8840 | 5 | 29444 | - |
| writes between units, clear in a compaction and in a seal | seal install | 8144 | 55274 | 36082 | 36528 | 4 | 67274 | - |
| writes between units, clear in a compaction and in a seal | seal last | 77 | 37070 | 19716 | 42410 | 6 | 61070 | - |
| writes between units, clear in a compaction and in a seal | seal layout | 3 | 32631 | 22544 | 32920 | 3 | 44631 | - |
| writes between units, clear in a compaction and in a seal | seal pass1 | 1513 | 47290 | 25360 | 44630 | 7 | 63290 | - |
| writes between units, clear in a compaction and in a seal | seal snapshot | 67 | 23448 | 18361 | 33050 | 4 | 39448 | - |
| writes between units, clear in a compaction and in a seal | seal write | 1577 | 51157 | 33111 | 44228 | 5 | 68218 | - |
| 170 running records: seal, compaction of patches and deletes, column release | drain | 642 | 38056 | 25284 | 31640 | 7 | 54056 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release begin | 20 | 26811 | 18214 | 35912 | 5 | 46811 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release flip | 1 | 5616 | 5616 | 8200 | 3 | 17616 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release install | 56 | 26973 | 25250 | 32136 | 4 | 42973 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release last | 231 | 37004 | 31462 | 31280 | 4 | 45321 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release layout | 29 | 32181 | 30043 | 32920 | 3 | 44181 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release pass1 | 17092 | 31695 | 24671 | 35552 | 6 | 49003 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release snapshot | 1 | 15672 | 15672 | 19300 | 7 | 43672 | - |
| 170 running records: seal, compaction of patches and deletes, column release | release write | 43673 | 47952 | 33404 | 37480 | 5 | 60844 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal flip | 1 | 36948 | 36948 | 21880 | 5 | 56948 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal install | 114 | 39935 | 28309 | 31488 | 9 | 75677 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal last | 178 | 33647 | 20196 | 42000 | 5 | 51666 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal layout | 43 | 32003 | 30620 | 32920 | 3 | 44003 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal pass1 | 4132 | 40948 | 22659 | 37389 | 8 | 69989 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal snapshot | 4 | 24007 | 19578 | 34250 | 4 | 36426 | - |
| 170 running records: seal, compaction of patches and deletes, column release | seal write | 5449 | 54381 | 32118 | 43014 | 6 | 72554 | - |
| 120 Bool and tag leaves: seal, a group dropped | drain | 122 | 22986 | 11839 | 24520 | 7 | 42636 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal begin | 2 | 13278 | 12923 | 11912 | 5 | 33278 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal flip | 2 | 13633 | 13488 | 12040 | 5 | 33633 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal install | 188 | 34107 | 26206 | 39312 | 9 | 70107 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal last | 32 | 27087 | 26624 | 31240 | 6 | 46481 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal layout | 62 | 31533 | 29316 | 32920 | 3 | 43533 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal pass1 | 3177 | 45112 | 23899 | 38160 | 8 | 69112 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal snapshot | 70 | 20652 | 14769 | 29150 | 4 | 36652 | - |
| 120 Bool and tag leaves: seal, a group dropped | seal write | 3228 | 54487 | 33063 | 41286 | 4 | 62487 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction begin | 1 | 15972 | 15972 | 12312 | 7 | 43972 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction flip | 1 | 21905 | 21905 | 23340 | 4 | 37905 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction install | 1 | 22471 | 22471 | 25656 | 2 | 30471 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction last | 228 | 26844 | 25147 | 30760 | 6 | 47757 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction layout | 1 | 23755 | 23755 | 24740 | 3 | 35755 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction pass1 | 89557 | 30710 | 26640 | 39224 | 3 | 38710 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction snapshot | 1 | 18678 | 18678 | 23280 | 7 | 46678 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction write | 1708 | 49045 | 33032 | 35664 | 5 | 65395 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | drain | 7631 | 30552 | 22960 | 28600 | 7 | 51054 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal begin | 1 | 13658 | 13658 | 11912 | 7 | 41658 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal flip | 1 | 8516 | 8516 | 8360 | 5 | 28516 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal install | 2 | 27136 | 22138 | 32056 | 2 | 35136 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal last | 513 | 31399 | 21809 | 29160 | 5 | 51399 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal layout | 1 | 17622 | 17622 | 18360 | 2 | 25622 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal pass1 | 133038 | 33867 | 28112 | 37852 | 2 | 39569 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal snapshot | 5 | 22790 | 22543 | 32750 | 4 | 38790 | - |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal write | 1369 | 49708 | 41293 | 31460 | 5 | 68682 | - |
| the reference row: seal | drain | 39 | 10473 | 9476 | 13340 | 7 | 37512 | - |
| the reference row: seal | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| the reference row: seal | seal flip | 1 | 9313 | 9313 | 8760 | 5 | 29313 | - |
| the reference row: seal | seal install | 11 | 23373 | 20756 | 34528 | 3 | 35373 | - |
| the reference row: seal | seal last | 2 | 31764 | 22429 | 31160 | 5 | 47764 | - |
| the reference row: seal | seal layout | 2 | 37964 | 28335 | 36200 | 3 | 45964 | - |
| the reference row: seal | seal pass1 | 970 | 34093 | 22039 | 38994 | 4 | 48242 | - |
| the reference row: seal | seal snapshot | 33 | 23582 | 18284 | 30850 | 4 | 39582 | - |
| the reference row: seal | seal write | 493 | 50239 | 33101 | 44816 | 5 | 64973 | - |
| every flat leaf kind: seal, compaction, column release, a drop | drain | 322 | 28984 | 14341 | 30680 | 7 | 54239 | - |
| every flat leaf kind: seal, compaction, column release, a drop | drop | 2 | 39626 | 39120 | 37000 | 5 | 59626 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release begin | 16 | 27486 | 16273 | 35912 | 5 | 47486 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release flip | 1 | 5529 | 5529 | 8200 | 3 | 17529 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release install | 26 | 56747 | 28915 | 35232 | 6 | 80747 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release last | 62 | 33912 | 30852 | 31400 | 6 | 49410 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release layout | 12 | 31661 | 29608 | 33000 | 3 | 42855 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release pass1 | 6114 | 43929 | 25038 | 43190 | 6 | 67929 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release snapshot | 1 | 14012 | 14012 | 19300 | 7 | 42012 | - |
| every flat leaf kind: seal, compaction, column release, a drop | release write | 7856 | 52728 | 32960 | 37480 | 4 | 60728 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal begin | 2 | 16023 | 14295 | 17312 | 8 | 48023 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal flip | 2 | 23786 | 20687 | 19520 | 6 | 47786 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal install | 62 | 53677 | 29723 | 37776 | 9 | 89677 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal last | 26 | 36045 | 26165 | 42290 | 6 | 57965 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal layout | 25 | 36063 | 30103 | 33800 | 3 | 45098 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal pass1 | 4026 | 49846 | 29440 | 42734 | 8 | 73846 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal snapshot | 62 | 22565 | 15575 | 34200 | 4 | 38565 | - |
| every flat leaf kind: seal, compaction, column release, a drop | seal write | 11821 | 65593 | 25327 | 43654 | 6 | 77593 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction begin | 2 | 17593 | 16197 | 17712 | 7 | 45593 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction flip | 2 | 10427 | 7923 | 13840 | 6 | 34427 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction install | 56 | 51848 | 27811 | 34680 | 6 | 75848 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction last | 171 | 57278 | 36791 | 31400 | 6 | 61278 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction layout | 26 | 36530 | 29844 | 33800 | 3 | 44530 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction pass1 | 17162 | 45221 | 24900 | 43190 | 6 | 65176 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction snapshot | 2 | 14139 | 12333 | 20000 | 6 | 38139 | - |
| every flat leaf kind, columns added and rows written in the seal's install | compaction write | 17566 | 57656 | 34801 | 37480 | 6 | 71451 | - |
| every flat leaf kind, columns added and rows written in the seal's install | drain | 442 | 28984 | 16893 | 31780 | 8 | 54239 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release begin | 16 | 27486 | 16360 | 35912 | 5 | 47486 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release flip | 1 | 5529 | 5529 | 8200 | 3 | 17529 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release install | 27 | 55456 | 28768 | 34664 | 6 | 79456 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release last | 62 | 33123 | 30519 | 31400 | 5 | 48411 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release layout | 13 | 31429 | 28693 | 33000 | 3 | 43429 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release pass1 | 6450 | 43923 | 24923 | 43190 | 6 | 67923 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release snapshot | 1 | 14036 | 14036 | 19300 | 7 | 42036 | - |
| every flat leaf kind, columns added and rows written in the seal's install | release write | 8252 | 52728 | 32806 | 37480 | 4 | 60728 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal begin | 2 | 16733 | 14650 | 17312 | 8 | 48733 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal flip | 2 | 23786 | 22790 | 19520 | 8 | 53794 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal install | 3131 | 60220 | 37317 | 39480 | 7 | 87519 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal last | 27 | 40455 | 25940 | 42290 | 8 | 72455 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal layout | 26 | 36063 | 30455 | 33800 | 5 | 44522 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal pass1 | 5276 | 50740 | 30120 | 42734 | 8 | 74740 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal snapshot | 62 | 23558 | 15593 | 35050 | 7 | 39822 | - |
| every flat leaf kind, columns added and rows written in the seal's install | seal write | 11578 | 66877 | 28615 | 43654 | 5 | 77593 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction begin | 1 | 22535 | 22535 | 19872 | 8 | 54535 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction flip | 1 | 21959 | 21959 | 23340 | 4 | 37959 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction install | 5 | 33468 | 29845 | 34680 | 2 | 41468 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction last | 798 | 37760 | 31055 | 30680 | 7 | 51182 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction layout | 3 | 32186 | 29850 | 32920 | 5 | 46568 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction pass1 | 10938 | 36196 | 26833 | 43193 | 6 | 50776 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction snapshot | 1 | 17117 | 17117 | 22160 | 7 | 45117 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction write | 14372 | 52175 | 36194 | 37480 | 5 | 60313 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | drain | 3463 | 28032 | 20377 | 32260 | 7 | 51704 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release begin | 14 | 26817 | 14529 | 35912 | 5 | 46817 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release flip | 1 | 5545 | 5545 | 8200 | 3 | 17545 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release install | 5 | 35828 | 30073 | 34560 | 2 | 43828 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release last | 400 | 36157 | 33586 | 31040 | 7 | 55328 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release layout | 2 | 30281 | 27556 | 32200 | 2 | 38281 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release pass1 | 7406 | 35626 | 27645 | 38424 | 2 | 43626 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release snapshot | 1 | 13436 | 13436 | 19300 | 7 | 41436 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release write | 8029 | 57866 | 43269 | 36292 | 5 | 77866 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal begin | 2 | 20282 | 16425 | 19472 | 8 | 52282 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal flip | 2 | 16260 | 12852 | 16640 | 6 | 40260 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal install | 23 | 29129 | 21598 | 39480 | 3 | 41129 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal last | 141 | 26669 | 24058 | 40800 | 5 | 43366 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal layout | 5 | 34960 | 25385 | 35480 | 7 | 52533 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal pass1 | 3216 | 42726 | 26951 | 43709 | 7 | 58726 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal snapshot | 54 | 23734 | 17786 | 33850 | 4 | 39734 | - |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal write | 1503 | 51824 | 35289 | 42834 | 5 | 64329 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction begin | 1 | 24135 | 24135 | 20992 | 8 | 56135 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction flip | 1 | 22700 | 22700 | 23340 | 4 | 38700 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction install | 6 | 36694 | 30684 | 32264 | 3 | 44694 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction last | 783 | 38636 | 32105 | 30560 | 5 | 43594 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction layout | 3 | 32425 | 30162 | 32600 | 3 | 44425 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction pass1 | 12409 | 42469 | 25213 | 43193 | 4 | 58422 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction snapshot | 1 | 21991 | 21991 | 25360 | 4 | 37991 | - |
| mixed rows, the table opened with a newer schema in its seal | compaction write | 26454 | 53503 | 33606 | 37480 | 5 | 65503 | - |
| mixed rows, the table opened with a newer schema in its seal | drain | 1355 | 27443 | 15787 | 32260 | 7 | 47443 | - |
| mixed rows, the table opened with a newer schema in its seal | release begin | 27 | 26824 | 14107 | 35912 | 5 | 46824 | - |
| mixed rows, the table opened with a newer schema in its seal | release flip | 1 | 5561 | 5561 | 8200 | 3 | 17561 | - |
| mixed rows, the table opened with a newer schema in its seal | release install | 6 | 38499 | 29406 | 32384 | 2 | 46499 | - |
| mixed rows, the table opened with a newer schema in its seal | release last | 392 | 50391 | 47776 | 30640 | 4 | 57027 | - |
| mixed rows, the table opened with a newer schema in its seal | release layout | 2 | 30438 | 27830 | 32280 | 3 | 38438 | - |
| mixed rows, the table opened with a newer schema in its seal | release pass1 | 9326 | 39567 | 29745 | 38424 | 4 | 48508 | - |
| mixed rows, the table opened with a newer schema in its seal | release snapshot | 1 | 13460 | 13460 | 19300 | 7 | 41460 | - |
| mixed rows, the table opened with a newer schema in its seal | release write | 14180 | 58869 | 45632 | 36304 | 5 | 77696 | - |
| mixed rows, the table opened with a newer schema in its seal | seal begin | 2 | 20501 | 16534 | 20592 | 8 | 52501 | - |
| mixed rows, the table opened with a newer schema in its seal | seal flip | 2 | 16260 | 12852 | 16640 | 6 | 40260 | - |
| mixed rows, the table opened with a newer schema in its seal | seal install | 23 | 29129 | 21549 | 39480 | 3 | 41129 | - |
| mixed rows, the table opened with a newer schema in its seal | seal last | 153 | 26851 | 18197 | 42210 | 5 | 44158 | - |
| mixed rows, the table opened with a newer schema in its seal | seal layout | 5 | 32543 | 24170 | 32920 | 3 | 44543 | - |
| mixed rows, the table opened with a newer schema in its seal | seal pass1 | 1110 | 39015 | 23488 | 43438 | 4 | 49234 | - |
| mixed rows, the table opened with a newer schema in its seal | seal snapshot | 62 | 23734 | 17650 | 33600 | 4 | 39734 | - |
| mixed rows, the table opened with a newer schema in its seal | seal write | 1335 | 57864 | 32802 | 43188 | 5 | 67092 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | drain | 72 | 24638 | 15634 | 25320 | 7 | 41381 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | drop | 2 | 35220 | 30954 | 35400 | 7 | 54689 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal begin | 2 | 13658 | 13468 | 11912 | 7 | 41658 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal flip | 2 | 8635 | 8635 | 8440 | 5 | 28635 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal install | 30 | 49756 | 30416 | 34680 | 6 | 73756 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal last | 2 | 33300 | 33240 | 27880 | 5 | 53300 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal layout | 4 | 31131 | 25153 | 32120 | 4 | 47131 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal pass1 | 11087 | 72501 | 37813 | 39792 | 9 | 104501 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal snapshot | 38 | 26398 | 20441 | 31800 | 4 | 42398 | - |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal write | 10666 | 54533 | 42252 | 42846 | 4 | 64860 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction begin | 1 | 23414 | 23414 | 20992 | 8 | 55414 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction flip | 1 | 22009 | 22009 | 23340 | 4 | 38009 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction install | 59 | 52875 | 46967 | 29720 | 6 | 76875 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction last | 797 | 35427 | 32671 | 29760 | 5 | 42093 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction layout | 2 | 31854 | 28832 | 32120 | 5 | 51854 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction pass1 | 68892 | 45676 | 29906 | 41224 | 7 | 70272 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction snapshot | 1 | 21241 | 21241 | 25360 | 4 | 37241 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction write | 11444 | 50858 | 44848 | 35480 | 5 | 67467 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | drain | 1576 | 32198 | 21400 | 31860 | 7 | 52198 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal flip | 1 | 9232 | 9232 | 8520 | 5 | 29232 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal install | 63 | 51925 | 43599 | 39480 | 8 | 83925 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal last | 1196 | 30643 | 28727 | 29760 | 7 | 43707 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal layout | 2 | 31312 | 17958 | 32120 | 5 | 51312 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal pass1 | 90472 | 47465 | 29828 | 38652 | 8 | 71280 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal snapshot | 27 | 23336 | 19923 | 32000 | 4 | 39336 | - |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal write | 8614 | 64667 | 50613 | 31764 | 5 | 82991 | - |
| 120 Bool and tag leaves: compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| 120 Bool and tag leaves: compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| 120 Bool and tag leaves: compaction, column release | compaction install | 91 | 40149 | 32305 | 34680 | 6 | 60062 | - |
| 120 Bool and tag leaves: compaction, column release | compaction last | 16 | 28143 | 27657 | 31240 | 5 | 43903 | - |
| 120 Bool and tag leaves: compaction, column release | compaction layout | 31 | 30837 | 29443 | 32520 | 3 | 42837 | - |
| 120 Bool and tag leaves: compaction, column release | compaction pass1 | 4981 | 40358 | 25039 | 34280 | 6 | 64358 | - |
| 120 Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 11467 | 11467 | 14600 | 3 | 23467 | - |
| 120 Bool and tag leaves: compaction, column release | compaction write | 4290 | 47343 | 34929 | 34869 | 4 | 58096 | - |
| 120 Bool and tag leaves: compaction, column release | drain | 98 | 24638 | 15504 | 25320 | 7 | 48360 | - |
| 120 Bool and tag leaves: compaction, column release | release begin | 21 | 28145 | 18190 | 35912 | 5 | 48145 | - |
| 120 Bool and tag leaves: compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| 120 Bool and tag leaves: compaction, column release | release install | 87 | 41772 | 32512 | 34992 | 6 | 60175 | - |
| 120 Bool and tag leaves: compaction, column release | release last | 15 | 29445 | 28144 | 31560 | 4 | 45445 | - |
| 120 Bool and tag leaves: compaction, column release | release layout | 30 | 30963 | 29564 | 32520 | 3 | 42963 | - |
| 120 Bool and tag leaves: compaction, column release | release pass1 | 4817 | 40471 | 27146 | 34280 | 6 | 64471 | - |
| 120 Bool and tag leaves: compaction, column release | release snapshot | 1 | 14976 | 14976 | 19300 | 7 | 42976 | - |
| 120 Bool and tag leaves: compaction, column release | release write | 4147 | 47858 | 37340 | 34869 | 4 | 58209 | - |
| 120 Bool and tag leaves: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 120 Bool and tag leaves: compaction, column release | seal flip | 1 | 13633 | 13633 | 12040 | 5 | 33633 | - |
| 120 Bool and tag leaves: compaction, column release | seal install | 94 | 34107 | 26250 | 39312 | 9 | 70107 | - |
| 120 Bool and tag leaves: compaction, column release | seal last | 16 | 27087 | 26615 | 31240 | 6 | 46481 | - |
| 120 Bool and tag leaves: compaction, column release | seal layout | 31 | 31533 | 30284 | 32920 | 3 | 43533 | - |
| 120 Bool and tag leaves: compaction, column release | seal pass1 | 3053 | 45112 | 23533 | 38160 | 8 | 69112 | - |
| 120 Bool and tag leaves: compaction, column release | seal snapshot | 35 | 20652 | 15232 | 29150 | 4 | 36652 | - |
| 120 Bool and tag leaves: compaction, column release | seal write | 3119 | 53930 | 32621 | 41286 | 4 | 61930 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 17568 | 17568 | 17712 | 8 | 49568 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction install | 89 | 37209 | 26573 | 32360 | 6 | 57573 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction last | 1241 | 29825 | 23918 | 31280 | 5 | 35255 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 44 | 32446 | 31550 | 32920 | 3 | 44446 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 24021 | 35550 | 22481 | 41208 | 6 | 55880 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 20346 | 20346 | 23280 | 7 | 48346 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction write | 108435 | 49206 | 28898 | 35480 | 6 | 62197 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | drain | 2415 | 38080 | 25326 | 32280 | 7 | 54080 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release begin | 24 | 28235 | 19490 | 35912 | 5 | 48235 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release install | 87 | 36624 | 25885 | 32360 | 6 | 57253 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release last | 1240 | 31847 | 29478 | 31560 | 5 | 44119 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release layout | 43 | 31861 | 31010 | 32920 | 3 | 43861 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 23395 | 35529 | 29287 | 40793 | 6 | 59529 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 15736 | 15736 | 19300 | 7 | 43736 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | release write | 88388 | 59446 | 41153 | 35804 | 6 | 83446 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 13692 | 13692 | 11912 | 7 | 41692 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 37351 | 37351 | 22040 | 5 | 57351 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal install | 99 | 41811 | 29597 | 31256 | 10 | 81811 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal last | 666 | 32147 | 19529 | 42870 | 6 | 46492 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal layout | 44 | 32341 | 30812 | 32920 | 3 | 44341 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 5552 | 40445 | 25396 | 42970 | 6 | 57432 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 3 | 26898 | 18963 | 33750 | 4 | 42898 | - |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal write | 10669 | 55000 | 32543 | 43330 | 6 | 72341 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 15806 | 15806 | 12312 | 7 | 43806 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction install | 51 | 39941 | 26969 | 34680 | 4 | 51941 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction last | 1842 | 31296 | 24720 | 31280 | 5 | 42665 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 24 | 32682 | 31673 | 32920 | 3 | 44682 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 22236 | 35126 | 22805 | 37224 | 6 | 51126 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 19226 | 19226 | 23280 | 7 | 47226 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction write | 100794 | 49073 | 29285 | 35480 | 6 | 62251 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | drain | 2105 | 41363 | 22952 | 32280 | 7 | 57363 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release begin | 19 | 28235 | 17775 | 35912 | 5 | 48235 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release install | 49 | 39266 | 26028 | 34680 | 4 | 51266 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release last | 1715 | 32563 | 29498 | 31580 | 4 | 48563 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release layout | 23 | 32097 | 31065 | 32920 | 3 | 44097 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 21265 | 34489 | 29434 | 40793 | 6 | 53021 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 14616 | 14616 | 19300 | 7 | 42616 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | release write | 80523 | 59446 | 41263 | 36172 | 6 | 83446 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 22580 | 22580 | 15800 | 5 | 42580 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal install | 63 | 41586 | 28571 | 33960 | 4 | 57586 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal last | 947 | 32899 | 20742 | 42840 | 5 | 52899 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal layout | 24 | 31877 | 30975 | 32920 | 3 | 43877 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 5192 | 40143 | 26592 | 43355 | 8 | 56546 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 31 | 19892 | 15822 | 32050 | 4 | 35892 | - |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal write | 9430 | 57780 | 34394 | 43586 | 6 | 81345 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 22046 | 22046 | 20992 | 8 | 54046 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction install | 36 | 39851 | 27096 | 34680 | 4 | 51851 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction last | 2122 | 32554 | 25328 | 31260 | 5 | 36554 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 17 | 32627 | 31557 | 32920 | 3 | 44627 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 20251 | 36693 | 23094 | 41193 | 6 | 60693 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 22045 | 22045 | 25360 | 4 | 38045 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction write | 90765 | 49387 | 29468 | 35480 | 6 | 63187 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | drain | 1794 | 38861 | 22063 | 32280 | 7 | 54861 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release begin | 17 | 28235 | 16891 | 35912 | 5 | 48235 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release install | 34 | 39260 | 25995 | 34680 | 4 | 51260 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release last | 1868 | 31602 | 29483 | 31000 | 5 | 44119 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release layout | 16 | 32042 | 30020 | 32920 | 3 | 44042 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 19001 | 40473 | 29375 | 40393 | 6 | 64473 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 14192 | 14192 | 19300 | 7 | 42192 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | release write | 70849 | 59446 | 41199 | 35780 | 6 | 83446 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 19070 | 19070 | 13400 | 5 | 39070 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal install | 48 | 47853 | 26140 | 37880 | 4 | 63853 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal last | 963 | 31267 | 20116 | 42840 | 6 | 51600 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal layout | 17 | 31877 | 29994 | 32920 | 3 | 43877 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 4560 | 40173 | 26476 | 43212 | 6 | 59585 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 32 | 19718 | 15786 | 32300 | 4 | 35718 | - |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal write | 7790 | 56778 | 34317 | 43508 | 6 | 75075 | - |
| a 200-case tag: a drop | drain | 24 | 11601 | 9438 | 14840 | 7 | 37512 | - |
| a 200-case tag: a drop | drop | 51 | 44440 | 33137 | 35400 | 5 | 56440 | - |
| a 200-case tag: a drop | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 200-case tag: a drop | seal flip | 1 | 8724 | 8724 | 8440 | 5 | 28724 | - |
| a 200-case tag: a drop | seal install | 5 | 27575 | 20642 | 34528 | 3 | 35575 | - |
| a 200-case tag: a drop | seal last | 1 | 27840 | 27840 | 23840 | 6 | 51840 | - |
| a 200-case tag: a drop | seal layout | 1 | 24689 | 24689 | 25240 | 3 | 36689 | - |
| a 200-case tag: a drop | seal pass1 | 1302 | 37158 | 31004 | 36210 | 4 | 53158 | - |
| a 200-case tag: a drop | seal snapshot | 23 | 24028 | 19733 | 30500 | 4 | 40028 | - |
| a 200-case tag: a drop | seal write | 1439 | 46914 | 38653 | 31166 | 4 | 58354 | - |
| a 200-case tag, every row updated: a drop | drain | 24 | 11601 | 9438 | 14840 | 7 | 37512 | - |
| a 200-case tag, every row updated: a drop | drop | 51 | 44884 | 34118 | 35400 | 5 | 56884 | - |
| a 200-case tag, every row updated: a drop | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 200-case tag, every row updated: a drop | seal flip | 1 | 8724 | 8724 | 8440 | 5 | 28724 | - |
| a 200-case tag, every row updated: a drop | seal install | 5 | 27575 | 20642 | 34528 | 3 | 35575 | - |
| a 200-case tag, every row updated: a drop | seal last | 1 | 27840 | 27840 | 23840 | 6 | 51840 | - |
| a 200-case tag, every row updated: a drop | seal layout | 1 | 24689 | 24689 | 25240 | 3 | 36689 | - |
| a 200-case tag, every row updated: a drop | seal pass1 | 1302 | 37158 | 31004 | 36210 | 4 | 53158 | - |
| a 200-case tag, every row updated: a drop | seal snapshot | 23 | 24028 | 19733 | 30500 | 4 | 40028 | - |
| a 200-case tag, every row updated: a drop | seal write | 1439 | 46914 | 38653 | 31166 | 4 | 58354 | - |
| a 200-case tag, a third of the rows updated: a drop | drain | 24 | 11601 | 9438 | 14840 | 7 | 37512 | - |
| a 200-case tag, a third of the rows updated: a drop | drop | 51 | 45106 | 34033 | 35400 | 5 | 57106 | - |
| a 200-case tag, a third of the rows updated: a drop | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 200-case tag, a third of the rows updated: a drop | seal flip | 1 | 8724 | 8724 | 8440 | 5 | 28724 | - |
| a 200-case tag, a third of the rows updated: a drop | seal install | 5 | 27575 | 20642 | 34528 | 3 | 35575 | - |
| a 200-case tag, a third of the rows updated: a drop | seal last | 1 | 27840 | 27840 | 23840 | 6 | 51840 | - |
| a 200-case tag, a third of the rows updated: a drop | seal layout | 1 | 24689 | 24689 | 25240 | 3 | 36689 | - |
| a 200-case tag, a third of the rows updated: a drop | seal pass1 | 1302 | 37158 | 31004 | 36210 | 4 | 53158 | - |
| a 200-case tag, a third of the rows updated: a drop | seal snapshot | 23 | 24028 | 19733 | 30500 | 4 | 40028 | - |
| a 200-case tag, a third of the rows updated: a drop | seal write | 1439 | 46914 | 38653 | 31166 | 4 | 58354 | - |
| a 200-case tag: compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 200-case tag: compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 200-case tag: compaction, column release | compaction install | 69 | 35487 | 32858 | 33840 | 2 | 43487 | - |
| a 200-case tag: compaction, column release | compaction last | 1 | 27516 | 27516 | 23840 | 5 | 47516 | - |
| a 200-case tag: compaction, column release | compaction layout | 1 | 24287 | 24287 | 25240 | 2 | 32287 | - |
| a 200-case tag: compaction, column release | compaction pass1 | 3590 | 37139 | 25396 | 35560 | 4 | 49427 | - |
| a 200-case tag: compaction, column release | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 200-case tag: compaction, column release | compaction write | 2307 | 47673 | 33254 | 37262 | 4 | 63374 | - |
| a 200-case tag: compaction, column release | drain | 25 | 11601 | 9303 | 14840 | 7 | 37512 | - |
| a 200-case tag: compaction, column release | release begin | 14 | 27481 | 14328 | 35912 | 5 | 47481 | - |
| a 200-case tag: compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 200-case tag: compaction, column release | release install | 69 | 35613 | 32881 | 33840 | 2 | 43613 | - |
| a 200-case tag: compaction, column release | release last | 1 | 27784 | 27784 | 24040 | 6 | 51784 | - |
| a 200-case tag: compaction, column release | release layout | 1 | 17876 | 17876 | 18440 | 2 | 25876 | - |
| a 200-case tag: compaction, column release | release pass1 | 2565 | 37265 | 27356 | 34280 | 4 | 49553 | - |
| a 200-case tag: compaction, column release | release snapshot | 1 | 13384 | 13384 | 19300 | 7 | 41384 | - |
| a 200-case tag: compaction, column release | release write | 1538 | 44527 | 34464 | 33814 | 4 | 60527 | - |
| a 200-case tag: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 200-case tag: compaction, column release | seal flip | 1 | 8724 | 8724 | 8440 | 5 | 28724 | - |
| a 200-case tag: compaction, column release | seal install | 5 | 27575 | 20642 | 34528 | 3 | 35575 | - |
| a 200-case tag: compaction, column release | seal last | 1 | 27840 | 27840 | 23840 | 6 | 51840 | - |
| a 200-case tag: compaction, column release | seal layout | 1 | 24689 | 24689 | 25240 | 3 | 36689 | - |
| a 200-case tag: compaction, column release | seal pass1 | 1302 | 37158 | 31004 | 36210 | 4 | 53158 | - |
| a 200-case tag: compaction, column release | seal snapshot | 23 | 24028 | 19733 | 30500 | 4 | 40028 | - |
| a 200-case tag: compaction, column release | seal write | 1439 | 46914 | 38653 | 31166 | 4 | 58354 | - |
| a 40-case tag: compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 40-case tag: compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 40-case tag: compaction, column release | compaction install | 15 | 39333 | 32694 | 35280 | 2 | 47333 | - |
| a 40-case tag: compaction, column release | compaction last | 1 | 27121 | 27121 | 23840 | 5 | 47121 | - |
| a 40-case tag: compaction, column release | compaction layout | 1 | 24287 | 24287 | 25240 | 2 | 32287 | - |
| a 40-case tag: compaction, column release | compaction pass1 | 3588 | 34732 | 25208 | 35560 | 2 | 42732 | - |
| a 40-case tag: compaction, column release | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 40-case tag: compaction, column release | compaction write | 1795 | 45201 | 36214 | 35589 | 4 | 61201 | - |
| a 40-case tag: compaction, column release | drain | 15 | 9579 | 9248 | 12000 | 7 | 37579 | - |
| a 40-case tag: compaction, column release | release begin | 14 | 27481 | 13685 | 35912 | 5 | 47481 | - |
| a 40-case tag: compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 40-case tag: compaction, column release | release install | 15 | 39459 | 32346 | 35280 | 2 | 47459 | - |
| a 40-case tag: compaction, column release | release last | 1 | 28272 | 28272 | 23740 | 8 | 60272 | - |
| a 40-case tag: compaction, column release | release layout | 1 | 17876 | 17876 | 18440 | 2 | 25876 | - |
| a 40-case tag: compaction, column release | release pass1 | 2563 | 34858 | 27095 | 35240 | 2 | 42858 | - |
| a 40-case tag: compaction, column release | release snapshot | 1 | 13384 | 13384 | 19300 | 7 | 41384 | - |
| a 40-case tag: compaction, column release | release write | 1026 | 49137 | 40182 | 35589 | 4 | 65137 | - |
| a 40-case tag: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 40-case tag: compaction, column release | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 40-case tag: compaction, column release | seal install | 5 | 24844 | 20287 | 34528 | 2 | 32844 | - |
| a 40-case tag: compaction, column release | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 40-case tag: compaction, column release | seal layout | 1 | 24413 | 24413 | 25240 | 2 | 32413 | - |
| a 40-case tag: compaction, column release | seal pass1 | 1299 | 39061 | 30233 | 34740 | 2 | 47061 | - |
| a 40-case tag: compaction, column release | seal snapshot | 18 | 26377 | 21057 | 30600 | 4 | 42377 | - |
| a 40-case tag: compaction, column release | seal write | 1097 | 48950 | 41927 | 37859 | 4 | 59565 | - |
| a 17-case tag: compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 17-case tag: compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 17-case tag: compaction, column release | compaction install | 8 | 38487 | 29920 | 34408 | 2 | 46487 | - |
| a 17-case tag: compaction, column release | compaction last | 1 | 27121 | 27121 | 23840 | 5 | 47121 | - |
| a 17-case tag: compaction, column release | compaction layout | 1 | 24287 | 24287 | 25240 | 2 | 32287 | - |
| a 17-case tag: compaction, column release | compaction pass1 | 3587 | 34471 | 25147 | 34280 | 2 | 42471 | - |
| a 17-case tag: compaction, column release | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 17-case tag: compaction, column release | compaction write | 1795 | 48462 | 39263 | 34389 | 4 | 64462 | - |
| a 17-case tag: compaction, column release | drain | 14 | 9579 | 9300 | 12000 | 7 | 37579 | - |
| a 17-case tag: compaction, column release | release begin | 14 | 27481 | 13685 | 35912 | 5 | 47481 | - |
| a 17-case tag: compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 17-case tag: compaction, column release | release install | 7 | 38613 | 32069 | 34408 | 2 | 46613 | - |
| a 17-case tag: compaction, column release | release last | 1 | 28272 | 28272 | 23740 | 8 | 60272 | - |
| a 17-case tag: compaction, column release | release layout | 1 | 17876 | 17876 | 18440 | 2 | 25876 | - |
| a 17-case tag: compaction, column release | release pass1 | 2562 | 32415 | 27013 | 34280 | 2 | 40415 | - |
| a 17-case tag: compaction, column release | release snapshot | 1 | 13384 | 13384 | 19300 | 7 | 41384 | - |
| a 17-case tag: compaction, column release | release write | 1026 | 48585 | 45512 | 34389 | 4 | 64585 | - |
| a 17-case tag: compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 17-case tag: compaction, column release | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 17-case tag: compaction, column release | seal install | 5 | 23340 | 19986 | 34528 | 2 | 31340 | - |
| a 17-case tag: compaction, column release | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 17-case tag: compaction, column release | seal layout | 1 | 24413 | 24413 | 25240 | 2 | 32413 | - |
| a 17-case tag: compaction, column release | seal pass1 | 366 | 38319 | 22699 | 38956 | 2 | 46319 | - |
| a 17-case tag: compaction, column release | seal snapshot | 17 | 29477 | 21573 | 33800 | 4 | 45477 | - |
| a 17-case tag: compaction, column release | seal write | 164 | 54610 | 36301 | 37488 | 4 | 62610 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction begin | 2 | 18132 | 17960 | 17712 | 7 | 46132 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction flip | 2 | 10481 | 10481 | 13840 | 6 | 34481 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction install | 52 | 52835 | 45317 | 34040 | 8 | 82588 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction last | 2 | 28747 | 24861 | 21320 | 5 | 48747 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction layout | 2 | 31620 | 31620 | 28520 | 5 | 51620 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction pass1 | 68308 | 47876 | 33048 | 39224 | 8 | 79491 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction snapshot | 2 | 13734 | 13734 | 20000 | 6 | 37734 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction write | 13476 | 49173 | 43415 | 37480 | 6 | 71488 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | drain | 847 | 30488 | 18349 | 31880 | 7 | 58488 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal flip | 1 | 9087 | 9087 | 8520 | 5 | 29087 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal install | 17591 | 64147 | 49286 | 35760 | 8 | 83001 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal last | 1 | 34167 | 34167 | 21320 | 6 | 58167 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal layout | 1 | 30873 | 30873 | 28520 | 5 | 50873 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal pass1 | 22317 | 49789 | 41728 | 42840 | 8 | 74793 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal snapshot | 9 | 21457 | 20966 | 31750 | 4 | 33292 | - |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal write | 3901 | 61225 | 49846 | 34720 | 7 | 77294 | - |
| narrow rows, a Nat key and two Nats: seal | drain | 20 | 9579 | 9447 | 12000 | 7 | 37579 | - |
| narrow rows, a Nat key and two Nats: seal | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| narrow rows, a Nat key and two Nats: seal | seal flip | 1 | 8833 | 8833 | 8440 | 5 | 28833 | - |
| narrow rows, a Nat key and two Nats: seal | seal install | 5 | 22715 | 20047 | 34528 | 2 | 30715 | - |
| narrow rows, a Nat key and two Nats: seal | seal last | 1 | 26957 | 26957 | 23840 | 6 | 50957 | - |
| narrow rows, a Nat key and two Nats: seal | seal layout | 1 | 24350 | 24350 | 25240 | 2 | 32350 | - |
| narrow rows, a Nat key and two Nats: seal | seal pass1 | 496 | 30407 | 21373 | 35920 | 2 | 38407 | - |
| narrow rows, a Nat key and two Nats: seal | seal snapshot | 21 | 23978 | 20607 | 31500 | 4 | 39978 | - |
| narrow rows, a Nat key and two Nats: seal | seal write | 144 | 42587 | 31594 | 31402 | 4 | 58587 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction install | 13 | 37116 | 27702 | 31680 | 2 | 45116 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction last | 1 | 27121 | 27121 | 23840 | 5 | 47121 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction layout | 1 | 23369 | 23369 | 21640 | 2 | 31369 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction pass1 | 3188 | 34566 | 25582 | 34280 | 2 | 42566 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction write | 913 | 52985 | 43352 | 34389 | 4 | 64985 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | drain | 14 | 9579 | 9300 | 12000 | 7 | 37579 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal install | 5 | 23340 | 19986 | 34528 | 2 | 31340 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal layout | 1 | 23441 | 23441 | 21640 | 2 | 31441 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal pass1 | 366 | 37896 | 23208 | 38956 | 2 | 45896 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal snapshot | 17 | 29477 | 21573 | 33800 | 4 | 45477 | - |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal write | 94 | 57280 | 40979 | 30136 | 4 | 65280 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction install | 11 | 33931 | 24770 | 32424 | 2 | 41931 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction last | 1 | 28500 | 28500 | 23840 | 5 | 48500 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction layout | 1 | 23369 | 23369 | 21640 | 2 | 31369 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction pass1 | 3188 | 34566 | 25582 | 34280 | 2 | 42566 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction write | 913 | 52985 | 43352 | 34389 | 4 | 64985 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | drain | 14 | 9579 | 9294 | 12000 | 7 | 37579 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal install | 5 | 23340 | 19986 | 34528 | 2 | 31340 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal layout | 1 | 23441 | 23441 | 21640 | 2 | 31441 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal pass1 | 366 | 37896 | 23208 | 38956 | 2 | 45896 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal snapshot | 17 | 29477 | 21573 | 33800 | 4 | 45477 | - |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal write | 94 | 57280 | 40979 | 30136 | 4 | 65280 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction install | 9 | 47743 | 31548 | 33600 | 2 | 55743 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction last | 1 | 28500 | 28500 | 23840 | 5 | 48500 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction layout | 1 | 23369 | 23369 | 21640 | 2 | 31369 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction pass1 | 3188 | 31836 | 25587 | 34280 | 2 | 39836 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction write | 913 | 48246 | 39190 | 33189 | 4 | 64246 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | drain | 14 | 9579 | 9294 | 12000 | 7 | 37579 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal install | 5 | 23196 | 19958 | 34528 | 2 | 31196 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal layout | 1 | 23441 | 23441 | 21640 | 2 | 31441 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal pass1 | 366 | 38045 | 23246 | 38956 | 2 | 46045 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal snapshot | 17 | 29477 | 21573 | 33800 | 4 | 45477 | - |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal write | 94 | 53321 | 36150 | 29920 | 4 | 61321 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction install | 28 | 49406 | 33429 | 36640 | 4 | 57406 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction last | 1 | 28500 | 28500 | 23840 | 5 | 48500 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction layout | 1 | 23369 | 23369 | 21640 | 2 | 31369 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction pass1 | 3189 | 30673 | 25673 | 34280 | 2 | 38673 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction write | 913 | 57848 | 49554 | 35589 | 4 | 65848 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | drain | 16 | 9579 | 9197 | 12000 | 7 | 37579 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal begin | 1 | 13692 | 13692 | 11912 | 7 | 41692 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal flip | 1 | 8684 | 8684 | 8440 | 5 | 28684 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal install | 5 | 24716 | 20262 | 34528 | 2 | 32716 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal last | 1 | 26966 | 26966 | 23840 | 6 | 50966 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal layout | 1 | 23441 | 23441 | 21640 | 2 | 31441 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal pass1 | 1299 | 40647 | 30521 | 35980 | 2 | 48647 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal snapshot | 19 | 27827 | 20432 | 32000 | 4 | 43827 | - |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal write | 1027 | 63689 | 54228 | 32292 | 4 | 71689 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction install | 78 | 51104 | 43499 | 34680 | 6 | 68208 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction last | 1 | 28895 | 28895 | 23840 | 5 | 48895 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction layout | 1 | 23369 | 23369 | 21640 | 2 | 31369 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction pass1 | 3192 | 38748 | 25798 | 34280 | 4 | 51137 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction write | 913 | 47063 | 39347 | 36789 | 4 | 62641 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | drain | 15 | 11601 | 9329 | 14840 | 7 | 37512 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal flip | 1 | 8656 | 8656 | 8360 | 5 | 28656 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal install | 5 | 28474 | 20744 | 34528 | 3 | 36474 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal last | 1 | 27840 | 27840 | 23840 | 6 | 51840 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal layout | 1 | 23717 | 23717 | 21640 | 3 | 35717 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal pass1 | 1302 | 43112 | 30932 | 36060 | 4 | 59112 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal write | 1368 | 46790 | 38538 | 31062 | 4 | 57154 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction install | 10 | 52668 | 34648 | 33888 | 6 | 76668 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction last | 1 | 30240 | 30240 | 23840 | 5 | 50240 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction layout | 1 | 23797 | 23797 | 21640 | 3 | 35797 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction pass1 | 4473 | 69473 | 29085 | 35560 | 8 | 101473 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction write | 1370 | 51355 | 36096 | 34176 | 4 | 65275 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | drain | 50 | 25211 | 18394 | 25320 | 7 | 49211 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction install | 177 | 50288 | 42570 | 34888 | 6 | 69719 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction last | 1 | 30240 | 30240 | 23840 | 5 | 50240 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction layout | 1 | 23797 | 23797 | 21640 | 3 | 35797 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction pass1 | 4473 | 69473 | 29085 | 35560 | 8 | 101473 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction write | 1370 | 51355 | 36096 | 34176 | 4 | 65275 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | drain | 50 | 25127 | 18392 | 25320 | 7 | 49127 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction begin | 2 | 17789 | 16295 | 17712 | 7 | 45789 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction flip | 2 | 10522 | 7951 | 13840 | 6 | 34522 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction install | 1022 | 54091 | 47207 | 36720 | 8 | 85553 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction last | 2 | 33169 | 28669 | 23840 | 7 | 61169 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction layout | 2 | 24508 | 24152 | 21640 | 3 | 36508 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction pass1 | 10317 | 74645 | 29281 | 35560 | 11 | 110645 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction snapshot | 2 | 13730 | 11802 | 20000 | 6 | 37730 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction write | 2744 | 51355 | 36965 | 39820 | 4 | 65959 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | drain | 120 | 24638 | 17964 | 27380 | 7 | 47858 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction begin | 2 | 18448 | 16625 | 17712 | 7 | 46448 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction flip | 2 | 10773 | 8077 | 14080 | 6 | 34773 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction install | 1281 | 55456 | 48674 | 37440 | 7 | 76594 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction last | 2 | 36221 | 30328 | 23840 | 6 | 60221 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction layout | 2 | 25329 | 24563 | 21640 | 4 | 41329 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction pass1 | 10317 | 74645 | 29302 | 36264 | 11 | 110645 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction snapshot | 2 | 13730 | 11802 | 20000 | 6 | 37730 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction write | 2744 | 51355 | 37127 | 39820 | 4 | 66760 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | drain | 136 | 23858 | 17665 | 27100 | 7 | 47858 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction begin | 1 | 19119 | 19119 | 17712 | 7 | 47119 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction flip | 1 | 11749 | 11749 | 14320 | 6 | 35749 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction install | 7 | 53243 | 41386 | 31008 | 6 | 77243 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction last | 1 | 24931 | 24931 | 24040 | 6 | 48931 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction layout | 1 | 22548 | 22548 | 23840 | 6 | 46548 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction pass1 | 7397 | 73063 | 29640 | 34080 | 10 | 101244 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction snapshot | 1 | 14408 | 14408 | 20000 | 6 | 38408 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction write | 2058 | 46182 | 38568 | 35292 | 6 | 65353 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | drain | 120 | 24638 | 17928 | 27380 | 7 | 45295 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release begin | 14 | 27481 | 14328 | 35912 | 5 | 47481 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release install | 1014 | 54217 | 47425 | 36720 | 10 | 89512 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release last | 1 | 34889 | 34889 | 22140 | 6 | 58889 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release layout | 1 | 18141 | 18141 | 18440 | 3 | 30141 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release pass1 | 5341 | 64387 | 29775 | 34504 | 8 | 96387 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release snapshot | 1 | 14143 | 14143 | 19300 | 7 | 42143 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release write | 2053 | 50122 | 36109 | 31204 | 4 | 62122 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction begin | 1 | 17789 | 17789 | 17712 | 7 | 45789 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction flip | 1 | 9271 | 9271 | 13600 | 6 | 33271 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction install | 7 | 52327 | 41372 | 31008 | 6 | 76327 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction last | 1 | 24169 | 24169 | 23840 | 5 | 44169 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction layout | 1 | 24562 | 24562 | 21640 | 3 | 36562 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction pass1 | 8764 | 72385 | 28702 | 34680 | 10 | 109874 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction snapshot | 1 | 13730 | 13730 | 20000 | 6 | 37730 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction write | 2058 | 47425 | 37924 | 35292 | 4 | 61477 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | drain | 91 | 24638 | 17438 | 28600 | 7 | 45251 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal install | 2024 | 68357 | 63888 | 35928 | 8 | 96357 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal last | 1 | 32451 | 32451 | 18040 | 6 | 56451 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal pass1 | 2679 | 61142 | 39144 | 35980 | 7 | 86051 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | drain | 26 | 22986 | 15080 | 24520 | 7 | 42134 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal install | 528 | 63305 | 59164 | 35928 | 10 | 97754 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal last | 1 | 30085 | 30085 | 23840 | 6 | 54085 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal pass1 | 2679 | 61142 | 39145 | 35980 | 9 | 86051 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction begin | 3 | 22705 | 18806 | 20992 | 8 | 54705 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction flip | 3 | 21943 | 14375 | 23340 | 6 | 37943 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction install | 672 | 58510 | 52438 | 34544 | 8 | 84580 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction last | 3223 | 34508 | 31967 | 29760 | 6 | 55201 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction layout | 3 | 31844 | 31779 | 28520 | 5 | 51844 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction pass1 | 532744 | 47535 | 31361 | 43620 | 8 | 73275 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction snapshot | 3 | 21241 | 16653 | 25360 | 6 | 38985 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction write | 38526 | 54396 | 44284 | 37480 | 7 | 78405 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | drain | 4180 | 29064 | 22071 | 30500 | 7 | 49064 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release begin | 14 | 27502 | 14382 | 35912 | 5 | 47502 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release flip | 1 | 5545 | 5545 | 8200 | 3 | 17545 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release install | 30 | 51993 | 45930 | 29304 | 6 | 75993 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release last | 399 | 35728 | 33601 | 29640 | 5 | 41153 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release layout | 1 | 24108 | 24108 | 21720 | 4 | 40108 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release pass1 | 42325 | 45235 | 31842 | 38424 | 6 | 63453 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release write | 3693 | 56277 | 52271 | 36280 | 5 | 71843 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal flip | 1 | 9166 | 9166 | 8520 | 5 | 29166 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal install | 6350 | 51176 | 46381 | 39480 | 8 | 81460 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal last | 1206 | 39379 | 28504 | 29960 | 6 | 63379 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal layout | 1 | 31042 | 31042 | 28520 | 4 | 47042 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal pass1 | 173131 | 46270 | 29807 | 37852 | 8 | 71253 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal snapshot | 23 | 26899 | 19771 | 33000 | 4 | 42899 | - |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal write | 6419 | 60327 | 50345 | 31764 | 5 | 75083 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction begin | 3 | 23231 | 19571 | 21392 | 8 | 55231 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction flip | 3 | 22009 | 14420 | 23340 | 6 | 38009 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction install | 1046 | 55227 | 52394 | 33688 | 8 | 82374 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction last | 3195 | 34524 | 32068 | 29760 | 6 | 43572 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction layout | 3 | 31844 | 31806 | 28520 | 5 | 51844 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction pass1 | 506971 | 45780 | 30789 | 41224 | 8 | 73185 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction snapshot | 3 | 21241 | 17070 | 25360 | 6 | 38985 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction write | 38635 | 53207 | 45063 | 37480 | 5 | 63714 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | drain | 3132 | 29983 | 21148 | 31800 | 7 | 49983 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release begin | 14 | 27532 | 14384 | 35912 | 5 | 47532 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release flip | 1 | 5561 | 5561 | 8200 | 3 | 17561 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release install | 45 | 52290 | 45646 | 29304 | 6 | 76290 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release last | 399 | 35758 | 33629 | 29640 | 5 | 41183 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release layout | 1 | 24108 | 24108 | 21720 | 4 | 40108 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release pass1 | 55182 | 45226 | 31325 | 38424 | 6 | 63510 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release write | 3719 | 57193 | 52898 | 36280 | 5 | 71829 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal flip | 1 | 9232 | 9232 | 8520 | 5 | 29232 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal install | 9699 | 54056 | 46586 | 39480 | 8 | 83925 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal last | 1196 | 35501 | 28598 | 29760 | 6 | 59501 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal layout | 1 | 31097 | 31097 | 28520 | 5 | 51097 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal pass1 | 155903 | 41506 | 29157 | 38350 | 8 | 71280 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal snapshot | 23 | 26899 | 19791 | 32950 | 4 | 42899 | - |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal write | 6436 | 61378 | 50649 | 31764 | 5 | 73378 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction begin | 1 | 22046 | 22046 | 20992 | 8 | 54046 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction flip | 1 | 21864 | 21864 | 23340 | 4 | 37864 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction install | 320 | 59925 | 52481 | 35416 | 8 | 85655 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction last | 1 | 22425 | 22425 | 21320 | 4 | 38425 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction layout | 1 | 31800 | 31800 | 28520 | 5 | 51800 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction pass1 | 20324 | 46055 | 36828 | 37224 | 8 | 72939 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction snapshot | 1 | 21241 | 21241 | 25360 | 4 | 37241 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction write | 9434 | 49829 | 40591 | 35480 | 6 | 67482 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | drain | 1201 | 28165 | 18492 | 29180 | 7 | 46636 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release begin | 14 | 27515 | 14383 | 35912 | 5 | 47515 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release install | 11 | 51970 | 43915 | 29304 | 6 | 75970 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release last | 1 | 27979 | 27979 | 23920 | 5 | 47979 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release layout | 1 | 24142 | 24142 | 21720 | 4 | 40142 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release pass1 | 18309 | 45618 | 31409 | 39224 | 7 | 69618 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release write | 4819 | 54773 | 41094 | 37480 | 6 | 71463 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal flip | 1 | 9087 | 9087 | 8520 | 5 | 29087 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal install | 2174 | 66020 | 61189 | 38448 | 8 | 94020 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal last | 1 | 27606 | 27606 | 21320 | 5 | 47606 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal layout | 1 | 31813 | 31813 | 28520 | 6 | 55813 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal pass1 | 23202 | 50985 | 41738 | 37852 | 9 | 82985 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal snapshot | 15 | 25949 | 19732 | 31950 | 4 | 41949 | - |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal write | 4876 | 62696 | 50245 | 32151 | 7 | 81424 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction flip | 1 | 5403 | 5403 | 8200 | 3 | 17403 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction install | 3 | 31450 | 24163 | 33848 | 2 | 39450 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction last | 1 | 26162 | 26162 | 21320 | 4 | 42162 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction layout | 1 | 30373 | 30373 | 28520 | 3 | 42373 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction pass1 | 7493 | 44139 | 29003 | 41745 | 4 | 52139 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction snapshot | 1 | 9879 | 9879 | 14600 | 3 | 21879 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction write | 5318 | 56465 | 48039 | 34304 | 5 | 67799 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | drain | 551 | 26284 | 20171 | 28520 | 8 | 46202 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release begin | 14 | 28199 | 14430 | 35912 | 5 | 48199 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release install | 2 | 28645 | 28393 | 28816 | 2 | 36645 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release last | 1 | 27909 | 27909 | 23920 | 5 | 47909 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release layout | 1 | 23784 | 23784 | 21720 | 3 | 35784 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release pass1 | 6073 | 45636 | 31768 | 41745 | 4 | 53636 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release write | 3705 | 59340 | 53783 | 34304 | 4 | 70662 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal flip | 1 | 9066 | 9066 | 8520 | 5 | 29066 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal install | 7 | 27245 | 22420 | 35072 | 4 | 38111 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal last | 1 | 25890 | 25890 | 21320 | 5 | 45890 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal layout | 1 | 30988 | 30988 | 28520 | 4 | 46988 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal pass1 | 3563 | 45312 | 34045 | 44048 | 6 | 64147 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal snapshot | 26 | 24732 | 18713 | 30350 | 4 | 40732 | - |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal write | 3327 | 64184 | 51192 | 32298 | 5 | 84184 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction flip | 1 | 5419 | 5419 | 8200 | 3 | 17419 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction install | 10 | 51874 | 40768 | 34184 | 6 | 75874 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction last | 1 | 26323 | 26323 | 21320 | 4 | 42323 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction layout | 1 | 30858 | 30858 | 28520 | 4 | 46858 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction pass1 | 7326 | 41158 | 29084 | 39360 | 7 | 69158 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction snapshot | 1 | 9879 | 9879 | 14600 | 3 | 21879 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction write | 5182 | 56995 | 49284 | 36292 | 5 | 76995 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | drain | 615 | 23846 | 19981 | 25360 | 7 | 44966 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release begin | 14 | 28206 | 14431 | 35912 | 5 | 48206 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release flip | 1 | 5529 | 5529 | 8200 | 3 | 17529 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release install | 9 | 51993 | 43581 | 29304 | 6 | 75993 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release last | 1 | 28032 | 28032 | 23920 | 5 | 48032 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release layout | 1 | 24132 | 24132 | 21720 | 4 | 40132 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release pass1 | 5946 | 41277 | 31548 | 39360 | 7 | 69277 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release write | 3617 | 59822 | 54575 | 36292 | 5 | 71822 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal begin | 1 | 13658 | 13658 | 11912 | 7 | 41658 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal flip | 1 | 9082 | 9082 | 8520 | 5 | 29082 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal install | 14 | 50699 | 34263 | 34528 | 6 | 74699 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal last | 1 | 25890 | 25890 | 21320 | 5 | 45890 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal layout | 2 | 30898 | 17751 | 28920 | 5 | 50898 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal pass1 | 3148 | 51067 | 34138 | 43615 | 7 | 75067 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal snapshot | 25 | 23582 | 18997 | 29800 | 4 | 39582 | - |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal write | 3342 | 62590 | 50767 | 41296 | 5 | 74590 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction flip | 1 | 5419 | 5419 | 8200 | 3 | 17419 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction install | 6 | 51874 | 37578 | 29304 | 6 | 75874 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction last | 1 | 26323 | 26323 | 21320 | 4 | 42323 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction layout | 1 | 30858 | 30858 | 28520 | 4 | 46858 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction pass1 | 7657 | 39920 | 29153 | 41745 | 6 | 62995 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction snapshot | 1 | 9879 | 9879 | 14600 | 3 | 21879 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction write | 5245 | 56986 | 47810 | 34328 | 5 | 76986 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | drain | 602 | 22986 | 20228 | 25360 | 7 | 46632 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release begin | 14 | 28206 | 14431 | 35912 | 5 | 48206 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release flip | 1 | 5529 | 5529 | 8200 | 3 | 17529 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release install | 6 | 51993 | 36377 | 29304 | 6 | 75993 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release last | 1 | 28032 | 28032 | 23920 | 5 | 48032 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release layout | 1 | 24132 | 24132 | 21720 | 4 | 40132 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release pass1 | 6262 | 42106 | 31887 | 41745 | 6 | 63114 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release write | 3656 | 59449 | 53419 | 36280 | 5 | 70759 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal flip | 1 | 9082 | 9082 | 8520 | 5 | 29082 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal install | 10 | 50699 | 30463 | 34528 | 6 | 74699 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal last | 1 | 25890 | 25890 | 21320 | 5 | 45890 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal layout | 1 | 30988 | 30988 | 28520 | 4 | 46988 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal pass1 | 3973 | 51125 | 34168 | 44128 | 8 | 75125 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal snapshot | 26 | 21464 | 18861 | 29150 | 4 | 37464 | - |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal write | 3335 | 64789 | 51396 | 32784 | 6 | 84789 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | drain | 384 | 24638 | 19969 | 25320 | 8 | 48558 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal begin | 2 | 18573 | 15925 | 11912 | 7 | 46573 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal flip | 2 | 6536 | 6536 | 8200 | 5 | 26536 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal install | 262 | 52900 | 38051 | 33520 | 9 | 87284 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal last | 32 | 27075 | 26621 | 31240 | 6 | 46485 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal layout | 54 | 52044 | 33380 | 32920 | 6 | 64044 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal pass1 | 38629 | 50987 | 35476 | 42718 | 9 | 77566 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal snapshot | 6 | 25377 | 19308 | 33750 | 4 | 41377 | - |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal write | 65129 | 61783 | 48179 | 40534 | 7 | 85783 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction flip | 1 | 5451 | 5451 | 8200 | 3 | 17451 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction install | 14 | 51886 | 41410 | 31184 | 6 | 75886 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction last | 1 | 26333 | 26333 | 21320 | 4 | 42333 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction layout | 2 | 31889 | 18237 | 32520 | 5 | 51889 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction pass1 | 10151 | 44675 | 30898 | 39360 | 7 | 69176 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction snapshot | 1 | 9879 | 9879 | 14600 | 3 | 21879 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction write | 9590 | 57809 | 48408 | 35928 | 5 | 69809 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | drain | 619 | 26475 | 19907 | 28940 | 7 | 41824 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release begin | 14 | 28216 | 14433 | 35912 | 5 | 48216 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release flip | 1 | 5561 | 5561 | 8200 | 3 | 17561 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release install | 13 | 52007 | 43744 | 29432 | 6 | 76007 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release last | 1 | 28044 | 28044 | 23920 | 5 | 48044 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release layout | 1 | 25997 | 25997 | 25720 | 4 | 41997 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release pass1 | 8748 | 48454 | 33363 | 39360 | 7 | 69297 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release write | 7998 | 60839 | 52440 | 36280 | 5 | 72839 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal flip | 1 | 9112 | 9112 | 8520 | 5 | 29112 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal install | 18 | 50699 | 36191 | 34528 | 7 | 74715 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal last | 1 | 25888 | 25888 | 21320 | 5 | 45888 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal layout | 2 | 31963 | 18283 | 32520 | 5 | 51963 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal pass1 | 5335 | 50966 | 34656 | 43939 | 8 | 82966 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal snapshot | 25 | 24382 | 19749 | 33550 | 4 | 40382 | - |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal write | 6530 | 61296 | 50597 | 39500 | 6 | 73296 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction install | 3 | 43862 | 32707 | 29304 | 4 | 59862 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction last | 1 | 28861 | 28861 | 23840 | 5 | 48861 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction layout | 1 | 23797 | 23797 | 21640 | 3 | 35797 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction pass1 | 4323 | 65751 | 28635 | 35560 | 7 | 93751 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction write | 1360 | 49275 | 35839 | 34176 | 4 | 65275 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | drain | 51 | 25595 | 18421 | 25360 | 7 | 53595 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release begin | 14 | 27481 | 14328 | 35912 | 5 | 47481 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release install | 3 | 43988 | 30270 | 29304 | 4 | 59988 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release last | 1 | 29051 | 29051 | 24040 | 6 | 53051 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release layout | 1 | 18087 | 18087 | 18440 | 3 | 30087 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release pass1 | 3418 | 65877 | 30982 | 34648 | 7 | 93877 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release snapshot | 1 | 13384 | 13384 | 19300 | 7 | 41384 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release write | 1360 | 51671 | 37324 | 32136 | 4 | 61603 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal install | 6 | 42861 | 26414 | 39480 | 4 | 58861 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal pass1 | 2487 | 60049 | 38604 | 35980 | 10 | 100049 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal write | 1370 | 50340 | 43325 | 32256 | 4 | 62340 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction flip | 1 | 5381 | 5381 | 8200 | 3 | 17381 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction install | 4 | 45413 | 32873 | 30192 | 6 | 69413 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction last | 1 | 28861 | 28861 | 23840 | 5 | 48861 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction layout | 1 | 23797 | 23797 | 21640 | 3 | 35797 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction pass1 | 4350 | 68032 | 28812 | 35560 | 8 | 100032 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction snapshot | 1 | 9875 | 9875 | 14600 | 3 | 21875 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction write | 1361 | 49654 | 36205 | 36220 | 4 | 65008 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | drain | 74 | 25353 | 19548 | 25320 | 7 | 49353 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release begin | 14 | 27481 | 14328 | 35912 | 5 | 47481 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release install | 3 | 43988 | 38408 | 29304 | 6 | 66413 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release last | 1 | 29051 | 29051 | 24040 | 6 | 53051 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release layout | 1 | 18087 | 18087 | 18440 | 3 | 30087 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release pass1 | 3445 | 68158 | 31188 | 34880 | 8 | 100158 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release snapshot | 1 | 13384 | 13384 | 19300 | 7 | 41384 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release write | 1360 | 48125 | 37708 | 31692 | 4 | 61336 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal install | 7 | 45433 | 27678 | 34528 | 6 | 69433 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal pass1 | 2551 | 64179 | 38892 | 42596 | 10 | 104179 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal write | 1371 | 49807 | 43907 | 32898 | 4 | 61807 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction begin | 2 | 18460 | 16631 | 17712 | 7 | 46460 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction flip | 2 | 9996 | 7688 | 13600 | 6 | 33996 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction install | 13 | 53005 | 42991 | 29304 | 6 | 77005 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction last | 2 | 29225 | 27078 | 24040 | 6 | 53225 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction layout | 2 | 22548 | 20254 | 23840 | 6 | 46548 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction pass1 | 11871 | 74891 | 30056 | 36344 | 11 | 106891 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction snapshot | 2 | 14408 | 12141 | 20000 | 6 | 38408 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction write | 3433 | 49338 | 38278 | 35292 | 4 | 62182 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | drain | 156 | 24638 | 18316 | 25760 | 7 | 45546 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release begin | 14 | 27481 | 14328 | 35912 | 5 | 47481 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release flip | 1 | 5507 | 5507 | 8200 | 3 | 17507 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release install | 4841 | 47087 | 44068 | 37440 | 6 | 69824 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release last | 1 | 31344 | 31344 | 14840 | 7 | 59344 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release layout | 1 | 18141 | 18141 | 18440 | 3 | 30141 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release pass1 | 5341 | 64387 | 29719 | 34504 | 8 | 96387 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release snapshot | 1 | 14093 | 14093 | 19300 | 7 | 42093 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release write | 2053 | 50122 | 36109 | 31204 | 4 | 62122 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal flip | 1 | 8694 | 8694 | 8360 | 5 | 28694 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal install | 7 | 42861 | 25688 | 34528 | 6 | 61429 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal last | 1 | 28706 | 28706 | 23840 | 6 | 52706 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal layout | 1 | 23869 | 23869 | 21640 | 3 | 35869 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal pass1 | 2679 | 61142 | 39033 | 35980 | 9 | 95515 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal snapshot | 16 | 22835 | 22767 | 31200 | 4 | 38410 | - |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal write | 1371 | 54264 | 43583 | 32256 | 4 | 66264 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction begin | 1 | 21146 | 21146 | 12312 | 7 | 49146 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction flip | 1 | 21665 | 21665 | 23340 | 4 | 37665 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction install | 176 | 40814 | 30065 | 33808 | 8 | 72814 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction last | 280 | 62108 | 42185 | 31240 | 5 | 66108 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction layout | 76 | 32778 | 31164 | 32920 | 5 | 51423 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction pass1 | 41291 | 43636 | 32663 | 41624 | 8 | 69207 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction snapshot | 1 | 25776 | 25776 | 23280 | 8 | 57776 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction write | 76664 | 63485 | 37566 | 36818 | 6 | 77118 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | drain | 1353 | 38627 | 23443 | 31640 | 7 | 54627 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release begin | 54 | 26811 | 20102 | 35912 | 5 | 46811 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release install | 101 | 45140 | 32551 | 35808 | 6 | 69140 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release last | 20 | 41508 | 28777 | 35560 | 4 | 45508 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release layout | 39 | 36788 | 30789 | 32920 | 5 | 51722 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release pass1 | 27350 | 48043 | 34518 | 40045 | 7 | 66683 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release snapshot | 1 | 21484 | 21484 | 19300 | 8 | 53484 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release write | 25491 | 65537 | 46109 | 37949 | 6 | 82201 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal begin | 1 | 17114 | 17114 | 16612 | 8 | 49114 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal flip | 1 | 28277 | 28277 | 15080 | 5 | 48277 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal install | 202 | 59620 | 36311 | 39904 | 10 | 87620 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal last | 149 | 33116 | 22646 | 42950 | 6 | 47240 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal layout | 76 | 33076 | 31537 | 32920 | 6 | 56488 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal pass1 | 27156 | 47517 | 34453 | 43489 | 9 | 74642 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal snapshot | 2 | 26546 | 21370 | 29950 | 5 | 42546 | - |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal write | 29490 | 61580 | 47314 | 43035 | 6 | 83539 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction begin | 1 | 14802 | 14802 | 12312 | 5 | 34802 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction flip | 1 | 5340 | 5340 | 8200 | 3 | 17340 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction install | 4 | 43852 | 29272 | 32752 | 4 | 59852 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction last | 1 | 26293 | 26293 | 21320 | 4 | 42293 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction layout | 1 | 30912 | 30912 | 28520 | 4 | 46912 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction pass1 | 13009 | 46629 | 29765 | 43178 | 7 | 74629 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction snapshot | 1 | 9879 | 9879 | 14600 | 3 | 21879 | - |
| Nat and Text values whose holders' compares run over their whole length | compaction write | 9799 | 49220 | 42926 | 37480 | 6 | 70823 | - |
| Nat and Text values whose holders' compares run over their whole length | drain | 614 | 29116 | 21524 | 30680 | 7 | 45365 | - |
| Nat and Text values whose holders' compares run over their whole length | release begin | 14 | 28231 | 14435 | 35912 | 5 | 48231 | - |
| Nat and Text values whose holders' compares run over their whole length | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| Nat and Text values whose holders' compares run over their whole length | release install | 3 | 43978 | 33692 | 29304 | 4 | 59978 | - |
| Nat and Text values whose holders' compares run over their whole length | release last | 1 | 28007 | 28007 | 23920 | 5 | 48007 | - |
| Nat and Text values whose holders' compares run over their whole length | release layout | 1 | 25141 | 25141 | 25320 | 4 | 41141 | - |
| Nat and Text values whose holders' compares run over their whole length | release pass1 | 11540 | 46755 | 30695 | 40378 | 7 | 74755 | - |
| Nat and Text values whose holders' compares run over their whole length | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| Nat and Text values whose holders' compares run over their whole length | release write | 7618 | 58915 | 51222 | 36304 | 6 | 72212 | - |
| Nat and Text values whose holders' compares run over their whole length | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| Nat and Text values whose holders' compares run over their whole length | seal flip | 1 | 8987 | 8987 | 8520 | 5 | 28987 | - |
| Nat and Text values whose holders' compares run over their whole length | seal install | 8 | 43121 | 24927 | 34528 | 4 | 59121 | - |
| Nat and Text values whose holders' compares run over their whole length | seal last | 1 | 25890 | 25890 | 21320 | 5 | 45890 | - |
| Nat and Text values whose holders' compares run over their whole length | seal layout | 1 | 31042 | 31042 | 28520 | 4 | 47042 | - |
| Nat and Text values whose holders' compares run over their whole length | seal pass1 | 7775 | 49770 | 37651 | 39338 | 6 | 73770 | - |
| Nat and Text values whose holders' compares run over their whole length | seal snapshot | 20 | 21507 | 21283 | 31800 | 4 | 33492 | - |
| Nat and Text values whose holders' compares run over their whole length | seal write | 3978 | 71480 | 48640 | 42238 | 7 | 84485 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction begin | 3 | 21776 | 19086 | 19872 | 8 | 53776 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction flip | 3 | 21864 | 14275 | 23340 | 6 | 37864 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction install | 3422 | 58466 | 54372 | 33728 | 8 | 85432 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction last | 3131 | 35190 | 32408 | 29760 | 6 | 43575 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction layout | 3 | 31844 | 31806 | 28520 | 5 | 51844 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction pass1 | 99780 | 46254 | 30400 | 39224 | 8 | 73105 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction snapshot | 3 | 17472 | 15814 | 22880 | 7 | 45472 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction write | 39293 | 54608 | 45160 | 37480 | 6 | 67873 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | drain | 1976 | 28255 | 15440 | 30680 | 7 | 55474 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release begin | 14 | 27515 | 14383 | 35912 | 5 | 47515 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release flip | 1 | 5466 | 5466 | 8200 | 3 | 17466 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release install | 116 | 51970 | 46608 | 31312 | 6 | 75970 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release last | 392 | 35865 | 34075 | 29560 | 5 | 47442 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release layout | 1 | 24108 | 24108 | 21720 | 4 | 40108 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release pass1 | 13102 | 45604 | 30979 | 38424 | 6 | 63430 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release snapshot | 1 | 13388 | 13388 | 19300 | 7 | 41388 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release write | 3813 | 57993 | 53092 | 36280 | 6 | 72960 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal begin | 1 | 12568 | 12568 | 11912 | 5 | 32568 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal flip | 1 | 9087 | 9087 | 8520 | 5 | 29087 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal install | 15541 | 60841 | 48037 | 35200 | 8 | 81295 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal last | 1174 | 34343 | 28793 | 29760 | 6 | 50343 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal layout | 1 | 31097 | 31097 | 28520 | 5 | 51097 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal pass1 | 25556 | 49923 | 30837 | 43082 | 8 | 73923 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal snapshot | 23 | 24782 | 19773 | 30500 | 4 | 40782 | - |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal write | 6644 | 61646 | 50187 | 31788 | 7 | 76661 | - |

Most instructions of any unit: 74891. Most changed pages: 11. Most charged: 110645.

Units cut by the replica's count as their host meter (instructions on the replica, by its cost model):

| scenario | job and step | units | most instructions | mean instructions | most estimated | most changed pages | most charged | longest between meter readings |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| mixed rows: seal, compaction of deletes and patches, a drop | compaction begin | 1 | 24166 | 24166 | 20992 | 8 | 56166 | 14065 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction flip | 1 | 50222 | 50222 | 45240 | 4 | 66222 | 5690 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction install | 3 | 62381 | 57324 | 63488 | 3 | 70381 | 9515 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction last | 203 | 65622 | 63100 | 59280 | 4 | 73057 | 6008 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction layout | 2 | 61218 | 55091 | 60040 | 2 | 69218 | 7562 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction pass1 | 2850 | 71425 | 65276 | 102088 | 2 | 75425 | 10405 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction snapshot | 1 | 50006 | 50006 | 47260 | 4 | 66006 | 5740 |
| mixed rows: seal, compaction of deletes and patches, a drop | compaction write | 5293 | 70812 | 64932 | 83804 | 5 | 88073 | 10064 |
| mixed rows: seal, compaction of deletes and patches, a drop | drain | 411 | 53044 | 41448 | 126340 | 7 | 68248 | 14956 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal begin | 2 | 21400 | 17087 | 20592 | 8 | 53400 | 11876 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal flip | 2 | 17276 | 13463 | 16720 | 6 | 41276 | 8715 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal install | 8 | 64660 | 55728 | 106952 | 4 | 72660 | 14232 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal last | 14 | 71311 | 67263 | 129430 | 8 | 95011 | 8371 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal layout | 3 | 61675 | 41451 | 60040 | 3 | 73675 | 7865 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal pass1 | 260 | 75019 | 68366 | 122060 | 4 | 87995 | 8653 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal snapshot | 16 | 62386 | 59453 | 120500 | 4 | 70386 | 9923 |
| mixed rows: seal, compaction of deletes and patches, a drop | seal write | 322 | 78113 | 71682 | 104630 | 6 | 97610 | 7766 |
| 64 KiB-class values: seal, compaction of patches | compaction begin | 1 | 22945 | 22945 | 20992 | 8 | 54945 | 13406 |
| 64 KiB-class values: seal, compaction of patches | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| 64 KiB-class values: seal, compaction of patches | compaction install | 1 | 49852 | 49852 | 51124 | 3 | 61852 | 5652 |
| 64 KiB-class values: seal, compaction of patches | compaction last | 1 | 33277 | 33277 | 30900 | 7 | 61277 | 5776 |
| 64 KiB-class values: seal, compaction of patches | compaction layout | 1 | 47348 | 47348 | 41160 | 3 | 59348 | 6703 |
| 64 KiB-class values: seal, compaction of patches | compaction pass1 | 51 | 72796 | 67192 | 100228 | 2 | 76796 | 11839 |
| 64 KiB-class values: seal, compaction of patches | compaction snapshot | 1 | 49954 | 49954 | 47260 | 4 | 65954 | 5740 |
| 64 KiB-class values: seal, compaction of patches | compaction write | 466 | 78553 | 66731 | 99780 | 8 | 110553 | 15191 |
| 64 KiB-class values: seal, compaction of patches | drain | 596 | 49021 | 31646 | 44860 | 7 | 69021 | 7193 |
| 64 KiB-class values: seal, compaction of patches | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 64 KiB-class values: seal, compaction of patches | seal flip | 1 | 8898 | 8898 | 8440 | 5 | 28898 | 900 |
| 64 KiB-class values: seal, compaction of patches | seal install | 1 | 49676 | 49676 | 49184 | 3 | 61676 | 4785 |
| 64 KiB-class values: seal, compaction of patches | seal last | 1 | 30100 | 30100 | 25340 | 6 | 54100 | 6948 |
| 64 KiB-class values: seal, compaction of patches | seal layout | 1 | 24718 | 24718 | 21640 | 2 | 32718 | 6703 |
| 64 KiB-class values: seal, compaction of patches | seal pass1 | 81 | 66068 | 62943 | 93088 | 2 | 72391 | 6354 |
| 64 KiB-class values: seal, compaction of patches | seal snapshot | 1 | 21967 | 21967 | 24200 | 4 | 37967 | 12409 |
| 64 KiB-class values: seal, compaction of patches | seal write | 557 | 75319 | 64313 | 44944 | 7 | 103319 | 9764 |
| 621 leaves: seal, compaction of 600 patch units | compaction begin | 1 | 19689 | 19689 | 12312 | 5 | 39689 | 865 |
| 621 leaves: seal, compaction of 600 patch units | compaction flip | 1 | 5458 | 5458 | 8200 | 3 | 17458 | 902 |
| 621 leaves: seal, compaction of 600 patch units | compaction install | 104 | 65171 | 62391 | 73448 | 6 | 87404 | 10010 |
| 621 leaves: seal, compaction of 600 patch units | compaction last | 35 | 61309 | 60751 | 64040 | 4 | 65309 | 6422 |
| 621 leaves: seal, compaction of 600 patch units | compaction layout | 69 | 67166 | 65096 | 67320 | 3 | 79166 | 7558 |
| 621 leaves: seal, compaction of 600 patch units | compaction pass1 | 30063 | 67434 | 65057 | 96560 | 9 | 100955 | 13378 |
| 621 leaves: seal, compaction of 600 patch units | compaction snapshot | 1 | 21068 | 21068 | 14600 | 5 | 41068 | 4192 |
| 621 leaves: seal, compaction of 600 patch units | compaction write | 23344 | 67207 | 65165 | 80510 | 5 | 86355 | 7478 |
| 621 leaves: seal, compaction of 600 patch units | drain | 236 | 46634 | 20465 | 42440 | 8 | 58634 | 8643 |
| 621 leaves: seal, compaction of 600 patch units | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 621 leaves: seal, compaction of 600 patch units | seal flip | 1 | 6805 | 6805 | 8200 | 5 | 26805 | 900 |
| 621 leaves: seal, compaction of 600 patch units | seal install | 156 | 64341 | 62648 | 65592 | 6 | 87191 | 10276 |
| 621 leaves: seal, compaction of 600 patch units | seal last | 34 | 63537 | 61661 | 67320 | 6 | 67537 | 6948 |
| 621 leaves: seal, compaction of 600 patch units | seal layout | 65 | 66465 | 63429 | 37800 | 3 | 78465 | 6802 |
| 621 leaves: seal, compaction of 600 patch units | seal pass1 | 2081 | 73793 | 70568 | 94020 | 9 | 104837 | 23444 |
| 621 leaves: seal, compaction of 600 patch units | seal snapshot | 8 | 60609 | 58377 | 113900 | 6 | 68609 | 9379 |
| 621 leaves: seal, compaction of 600 patch units | seal write | 274 | 68368 | 62942 | 53792 | 5 | 83873 | 5568 |
| values sharing long prefixes: seal, compaction | compaction begin | 1 | 23471 | 23471 | 21392 | 8 | 55471 | 13406 |
| values sharing long prefixes: seal, compaction | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| values sharing long prefixes: seal, compaction | compaction install | 2 | 61873 | 53610 | 63792 | 2 | 69873 | 5564 |
| values sharing long prefixes: seal, compaction | compaction last | 3 | 61848 | 55729 | 96860 | 6 | 69848 | 3857 |
| values sharing long prefixes: seal, compaction | compaction layout | 1 | 50163 | 50163 | 46160 | 4 | 66163 | 6663 |
| values sharing long prefixes: seal, compaction | compaction pass1 | 1438 | 74331 | 68954 | 118376 | 4 | 80593 | 17104 |
| values sharing long prefixes: seal, compaction | compaction snapshot | 1 | 49978 | 49978 | 47260 | 4 | 65978 | 5740 |
| values sharing long prefixes: seal, compaction | compaction write | 338 | 72397 | 66491 | 127080 | 8 | 98072 | 14242 |
| values sharing long prefixes: seal, compaction | drain | 423 | 50944 | 33124 | 46280 | 7 | 69021 | 11192 |
| values sharing long prefixes: seal, compaction | seal begin | 1 | 13898 | 13898 | 11912 | 7 | 41898 | 865 |
| values sharing long prefixes: seal, compaction | seal flip | 1 | 9272 | 9272 | 8600 | 5 | 29272 | 900 |
| values sharing long prefixes: seal, compaction | seal install | 2 | 63635 | 49420 | 54664 | 4 | 71635 | 4792 |
| values sharing long prefixes: seal, compaction | seal last | 9 | 64641 | 62446 | 56860 | 6 | 76641 | 7145 |
| values sharing long prefixes: seal, compaction | seal layout | 1 | 38178 | 38178 | 35400 | 3 | 50178 | 6701 |
| values sharing long prefixes: seal, compaction | seal pass1 | 1732 | 73716 | 64639 | 103232 | 4 | 89277 | 15300 |
| values sharing long prefixes: seal, compaction | seal snapshot | 1 | 32026 | 32026 | 45400 | 4 | 48026 | 9479 |
| values sharing long prefixes: seal, compaction | seal write | 621 | 70430 | 64103 | 48152 | 8 | 100517 | 14359 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction begin | 1 | 23807 | 23807 | 17712 | 8 | 55807 | 18729 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction flip | 1 | 5347 | 5347 | 8200 | 3 | 17347 | 902 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction install | 175 | 65668 | 62814 | 70296 | 8 | 94585 | 11898 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction last | 58 | 64307 | 60656 | 64040 | 5 | 76307 | 6422 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction layout | 115 | 68685 | 65689 | 67320 | 6 | 80685 | 7770 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction pass1 | 25974 | 69349 | 64228 | 92280 | 9 | 100816 | 11416 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction snapshot | 1 | 41911 | 41911 | 21000 | 10 | 81911 | 13018 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | compaction write | 42000 | 70525 | 63342 | 127004 | 6 | 90525 | 8588 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | drain | 293 | 54481 | 32277 | 126340 | 8 | 69296 | 10094 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | drop | 1 | 26354 | 26354 | 17400 | 6 | 50354 | 24290 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal begin | 2 | 17632 | 15748 | 17312 | 8 | 49632 | 12586 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal flip | 2 | 11235 | 8988 | 13600 | 8 | 43235 | 5891 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal install | 514 | 66209 | 62902 | 66712 | 9 | 99954 | 9661 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal last | 114 | 64197 | 61212 | 67320 | 6 | 68197 | 10156 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal layout | 236 | 68828 | 64308 | 67320 | 5 | 80345 | 8093 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal pass1 | 10966 | 74592 | 65033 | 127430 | 10 | 107917 | 21882 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal snapshot | 4 | 73097 | 48010 | 97350 | 9 | 109097 | 13439 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | seal write | 27006 | 78257 | 63913 | 218728 | 6 | 102257 | 8258 |
| rows of three schema versions: seal, column release, patches | drain | 183 | 54101 | 37529 | 54800 | 7 | 80949 | 10802 |
| rows of three schema versions: seal, column release, patches | release begin | 96 | 52338 | 40124 | 87760 | 6 | 68126 | 23796 |
| rows of three schema versions: seal, column release, patches | release flip | 1 | 5783 | 5783 | 8200 | 3 | 17783 | 897 |
| rows of three schema versions: seal, column release, patches | release install | 95 | 65522 | 61683 | 70248 | 6 | 87725 | 10379 |
| rows of three schema versions: seal, column release, patches | release last | 32 | 62612 | 60953 | 64840 | 4 | 66612 | 7979 |
| rows of three schema versions: seal, column release, patches | release layout | 66 | 67247 | 63919 | 67320 | 3 | 79247 | 11786 |
| rows of three schema versions: seal, column release, patches | release pass1 | 20797 | 70241 | 64390 | 90280 | 7 | 94373 | 14806 |
| rows of three schema versions: seal, column release, patches | release snapshot | 1 | 21540 | 21540 | 14600 | 5 | 41540 | 4200 |
| rows of three schema versions: seal, column release, patches | release write | 22655 | 70910 | 63254 | 72413 | 5 | 87987 | 13967 |
| rows of three schema versions: seal, column release, patches | seal begin | 1 | 16020 | 16020 | 11912 | 7 | 44020 | 13039 |
| rows of three schema versions: seal, column release, patches | seal flip | 1 | 8214 | 8214 | 8200 | 5 | 28214 | 6089 |
| rows of three schema versions: seal, column release, patches | seal install | 144 | 65318 | 62866 | 62392 | 6 | 89318 | 10276 |
| rows of three schema versions: seal, column release, patches | seal last | 32 | 64515 | 61635 | 67320 | 6 | 68515 | 7979 |
| rows of three schema versions: seal, column release, patches | seal layout | 66 | 68171 | 65615 | 67320 | 3 | 80171 | 11786 |
| rows of three schema versions: seal, column release, patches | seal pass1 | 16371 | 74854 | 64497 | 89200 | 9 | 99309 | 22665 |
| rows of three schema versions: seal, column release, patches | seal snapshot | 8 | 62934 | 59482 | 115500 | 6 | 70934 | 9329 |
| rows of three schema versions: seal, column release, patches | seal write | 17324 | 76061 | 65176 | 67688 | 5 | 88539 | 10331 |
| writes between units, clear in a compaction and in a seal | compaction begin | 1 | 18688 | 18688 | 17712 | 7 | 46688 | 13406 |
| writes between units, clear in a compaction and in a seal | compaction flip | 1 | 53640 | 53640 | 56100 | 4 | 69640 | 9617 |
| writes between units, clear in a compaction and in a seal | compaction install | 2 | 65598 | 60010 | 73024 | 2 | 73598 | 9473 |
| writes between units, clear in a compaction and in a seal | compaction last | 209 | 66043 | 62985 | 56280 | 7 | 72378 | 5987 |
| writes between units, clear in a compaction and in a seal | compaction layout | 2 | 61272 | 54211 | 60040 | 4 | 69272 | 10399 |
| writes between units, clear in a compaction and in a seal | compaction pass1 | 3226 | 71798 | 65679 | 112080 | 2 | 77591 | 11249 |
| writes between units, clear in a compaction and in a seal | compaction snapshot | 1 | 15382 | 15382 | 20000 | 6 | 39382 | 4993 |
| writes between units, clear in a compaction and in a seal | compaction write | 5439 | 75326 | 65842 | 99404 | 7 | 99326 | 11291 |
| writes between units, clear in a compaction and in a seal | drain | 738 | 54481 | 39805 | 63820 | 7 | 70156 | 13124 |
| writes between units, clear in a compaction and in a seal | seal begin | 2 | 13484 | 13129 | 11912 | 5 | 33484 | 865 |
| writes between units, clear in a compaction and in a seal | seal flip | 1 | 9650 | 9650 | 8840 | 5 | 29650 | 900 |
| writes between units, clear in a compaction and in a seal | seal install | 3536 | 81688 | 67251 | 77880 | 7 | 93688 | 19148 |
| writes between units, clear in a compaction and in a seal | seal last | 14 | 70804 | 67494 | 130070 | 6 | 81550 | 11535 |
| writes between units, clear in a compaction and in a seal | seal layout | 2 | 61928 | 33589 | 60040 | 3 | 73928 | 7933 |
| writes between units, clear in a compaction and in a seal | seal pass1 | 291 | 74949 | 68670 | 121440 | 4 | 90949 | 8653 |
| writes between units, clear in a compaction and in a seal | seal snapshot | 20 | 62049 | 58320 | 109300 | 4 | 70049 | 10809 |
| writes between units, clear in a compaction and in a seal | seal write | 437 | 78099 | 71276 | 104630 | 6 | 98099 | 6409 |
| 170 running records: seal, compaction of patches and deletes, column release | drain | 343 | 54210 | 46633 | 57680 | 7 | 71005 | 16412 |
| 170 running records: seal, compaction of patches and deletes, column release | release begin | 12 | 49801 | 44949 | 92140 | 8 | 65801 | 25397 |
| 170 running records: seal, compaction of patches and deletes, column release | release flip | 1 | 5822 | 5822 | 8200 | 3 | 17822 | 897 |
| 170 running records: seal, compaction of patches and deletes, column release | release install | 19 | 65122 | 62301 | 76728 | 4 | 81122 | 10743 |
| 170 running records: seal, compaction of patches and deletes, column release | release last | 112 | 65133 | 62519 | 80880 | 5 | 70533 | 8879 |
| 170 running records: seal, compaction of patches and deletes, column release | release layout | 14 | 67751 | 60554 | 66920 | 3 | 79751 | 12666 |
| 170 running records: seal, compaction of patches and deletes, column release | release pass1 | 4631 | 70065 | 65693 | 113097 | 6 | 87034 | 11132 |
| 170 running records: seal, compaction of patches and deletes, column release | release snapshot | 1 | 13402 | 13402 | 14600 | 3 | 25402 | 2354 |
| 170 running records: seal, compaction of patches and deletes, column release | release write | 14915 | 69245 | 64351 | 133804 | 5 | 87288 | 11260 |
| 170 running records: seal, compaction of patches and deletes, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 170 running records: seal, compaction of patches and deletes, column release | seal flip | 1 | 37154 | 37154 | 21880 | 5 | 57154 | 900 |
| 170 running records: seal, compaction of patches and deletes, column release | seal install | 49 | 77970 | 62987 | 65592 | 9 | 103671 | 19355 |
| 170 running records: seal, compaction of patches and deletes, column release | seal last | 38 | 71354 | 66481 | 125660 | 6 | 79354 | 6948 |
| 170 running records: seal, compaction of patches and deletes, column release | seal layout | 20 | 67079 | 64109 | 66920 | 3 | 79079 | 7605 |
| 170 running records: seal, compaction of patches and deletes, column release | seal pass1 | 849 | 73870 | 70342 | 110180 | 8 | 96609 | 20672 |
| 170 running records: seal, compaction of patches and deletes, column release | seal snapshot | 2 | 65381 | 38140 | 91700 | 4 | 81381 | 10229 |
| 170 running records: seal, compaction of patches and deletes, column release | seal write | 1621 | 80750 | 71196 | 91030 | 6 | 104750 | 6196 |
| 120 Bool and tag leaves: seal, a group dropped | drain | 113 | 47426 | 13528 | 42840 | 7 | 63426 | 8643 |
| 120 Bool and tag leaves: seal, a group dropped | seal begin | 2 | 13484 | 13129 | 11912 | 5 | 33484 | 865 |
| 120 Bool and tag leaves: seal, a group dropped | seal flip | 2 | 13839 | 13694 | 12040 | 5 | 33839 | 900 |
| 120 Bool and tag leaves: seal, a group dropped | seal install | 70 | 63758 | 61307 | 79416 | 7 | 90273 | 6984 |
| 120 Bool and tag leaves: seal, a group dropped | seal last | 14 | 63266 | 59753 | 67320 | 5 | 70110 | 6948 |
| 120 Bool and tag leaves: seal, a group dropped | seal layout | 30 | 63794 | 59071 | 66920 | 3 | 75794 | 7725 |
| 120 Bool and tag leaves: seal, a group dropped | seal pass1 | 706 | 74645 | 71123 | 99460 | 8 | 97269 | 19008 |
| 120 Bool and tag leaves: seal, a group dropped | seal snapshot | 16 | 61949 | 59865 | 118850 | 4 | 77519 | 9529 |
| 120 Bool and tag leaves: seal, a group dropped | seal write | 956 | 74626 | 70088 | 90126 | 4 | 90626 | 5799 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction begin | 1 | 16624 | 16624 | 12312 | 7 | 44624 | 14496 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction flip | 1 | 50168 | 50168 | 45240 | 4 | 66168 | 5690 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction install | 1 | 46713 | 46713 | 45176 | 3 | 58713 | 5652 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction last | 69 | 66118 | 63470 | 81280 | 6 | 82205 | 5434 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction layout | 1 | 46976 | 46976 | 42260 | 3 | 58976 | 6958 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction pass1 | 11241 | 70000 | 66168 | 118376 | 2 | 75305 | 10526 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction snapshot | 1 | 20278 | 20278 | 23280 | 7 | 48278 | 7708 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | compaction write | 533 | 71120 | 65587 | 85904 | 5 | 90241 | 9807 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | drain | 3795 | 55430 | 45836 | 53300 | 7 | 71374 | 16168 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal begin | 1 | 13864 | 13864 | 11912 | 7 | 41864 | 865 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal flip | 1 | 8722 | 8722 | 8360 | 5 | 28722 | 900 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal install | 1 | 39213 | 39213 | 50456 | 3 | 51213 | 4785 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal last | 94 | 63772 | 61750 | 89800 | 6 | 78931 | 7145 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal layout | 1 | 18968 | 18968 | 18360 | 2 | 26968 | 6927 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal pass1 | 19447 | 72560 | 62224 | 118390 | 2 | 80560 | 5214 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal snapshot | 2 | 64219 | 54373 | 88250 | 4 | 72219 | 11129 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | seal write | 873 | 66065 | 64239 | 46472 | 5 | 86065 | 6741 |
| the reference row: seal | drain | 39 | 11379 | 10130 | 13340 | 7 | 38171 | 7396 |
| the reference row: seal | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| the reference row: seal | seal flip | 1 | 9519 | 9519 | 8760 | 5 | 29519 | 900 |
| the reference row: seal | seal install | 3 | 61665 | 58155 | 100056 | 4 | 69665 | 4981 |
| the reference row: seal | seal last | 1 | 42789 | 42789 | 38460 | 6 | 66789 | 6948 |
| the reference row: seal | seal layout | 1 | 55336 | 55336 | 49960 | 3 | 67336 | 8799 |
| the reference row: seal | seal pass1 | 179 | 74273 | 70742 | 119331 | 4 | 87049 | 7183 |
| the reference row: seal | seal snapshot | 10 | 61349 | 56905 | 95150 | 4 | 69349 | 9459 |
| the reference row: seal | seal write | 145 | 76965 | 72167 | 98826 | 5 | 93253 | 9849 |
| every flat leaf kind: seal, compaction, column release, a drop | drain | 173 | 54625 | 25263 | 126340 | 8 | 70625 | 12827 |
| every flat leaf kind: seal, compaction, column release, a drop | drop | 2 | 67264 | 46782 | 49400 | 7 | 79264 | 17490 |
| every flat leaf kind: seal, compaction, column release, a drop | release begin | 8 | 48172 | 43023 | 92140 | 8 | 62552 | 26072 |
| every flat leaf kind: seal, compaction, column release, a drop | release flip | 1 | 5735 | 5735 | 8200 | 3 | 17735 | 897 |
| every flat leaf kind: seal, compaction, column release, a drop | release install | 11 | 67234 | 58384 | 76728 | 6 | 91234 | 9753 |
| every flat leaf kind: seal, compaction, column release, a drop | release last | 30 | 63727 | 60450 | 64360 | 6 | 71727 | 6409 |
| every flat leaf kind: seal, compaction, column release, a drop | release layout | 6 | 67261 | 57830 | 67080 | 4 | 82663 | 7806 |
| every flat leaf kind: seal, compaction, column release, a drop | release pass1 | 1391 | 71544 | 64178 | 145280 | 7 | 91620 | 14296 |
| every flat leaf kind: seal, compaction, column release, a drop | release snapshot | 1 | 11742 | 11742 | 14600 | 3 | 23742 | 2118 |
| every flat leaf kind: seal, compaction, column release, a drop | release write | 2681 | 71457 | 64202 | 168504 | 4 | 83260 | 10840 |
| every flat leaf kind: seal, compaction, column release, a drop | seal begin | 2 | 16922 | 14848 | 17312 | 8 | 48922 | 11876 |
| every flat leaf kind: seal, compaction, column release, a drop | seal flip | 2 | 24685 | 21239 | 19520 | 6 | 48685 | 16124 |
| every flat leaf kind: seal, compaction, column release, a drop | seal install | 28 | 74952 | 61661 | 73656 | 9 | 100979 | 19344 |
| every flat leaf kind: seal, compaction, column release, a drop | seal last | 10 | 71818 | 60888 | 88500 | 6 | 79818 | 10432 |
| every flat leaf kind: seal, compaction, column release, a drop | seal layout | 12 | 67807 | 61652 | 70280 | 7 | 79807 | 8323 |
| every flat leaf kind: seal, compaction, column release, a drop | seal pass1 | 1441 | 79087 | 63414 | 131080 | 8 | 103087 | 19817 |
| every flat leaf kind: seal, compaction, column release, a drop | seal snapshot | 16 | 62609 | 56801 | 119650 | 8 | 70609 | 9348 |
| every flat leaf kind: seal, compaction, column release, a drop | seal write | 2650 | 79857 | 64286 | 218728 | 5 | 93427 | 14337 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction begin | 2 | 18492 | 16750 | 17712 | 7 | 46492 | 13193 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction flip | 2 | 11326 | 8475 | 13840 | 6 | 35326 | 4549 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction install | 22 | 65060 | 59826 | 76728 | 6 | 87528 | 14623 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction last | 97 | 66722 | 63017 | 67320 | 4 | 74045 | 8665 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction layout | 13 | 67629 | 58802 | 67080 | 6 | 79300 | 7920 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction pass1 | 3705 | 74497 | 64464 | 149580 | 6 | 94591 | 18354 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction snapshot | 2 | 15739 | 13689 | 20000 | 6 | 39739 | 5185 |
| every flat leaf kind, columns added and rows written in the seal's install | compaction write | 6261 | 78880 | 64862 | 162904 | 6 | 96080 | 18206 |
| every flat leaf kind, columns added and rows written in the seal's install | drain | 241 | 54757 | 30073 | 126340 | 8 | 77138 | 12922 |
| every flat leaf kind, columns added and rows written in the seal's install | release begin | 8 | 48187 | 41586 | 92140 | 8 | 62552 | 26072 |
| every flat leaf kind, columns added and rows written in the seal's install | release flip | 1 | 5735 | 5735 | 8200 | 3 | 17735 | 897 |
| every flat leaf kind, columns added and rows written in the seal's install | release install | 11 | 67234 | 60339 | 76728 | 6 | 91234 | 9753 |
| every flat leaf kind, columns added and rows written in the seal's install | release last | 29 | 63868 | 61471 | 64360 | 4 | 73623 | 6541 |
| every flat leaf kind, columns added and rows written in the seal's install | release layout | 6 | 67146 | 60159 | 66920 | 4 | 79146 | 7623 |
| every flat leaf kind, columns added and rows written in the seal's install | release pass1 | 1473 | 71381 | 64052 | 145280 | 6 | 89375 | 14296 |
| every flat leaf kind, columns added and rows written in the seal's install | release snapshot | 1 | 11766 | 11766 | 14600 | 3 | 23766 | 2118 |
| every flat leaf kind, columns added and rows written in the seal's install | release write | 2798 | 73276 | 64159 | 168504 | 4 | 85276 | 10840 |
| every flat leaf kind, columns added and rows written in the seal's install | seal begin | 2 | 17632 | 15203 | 17312 | 8 | 49632 | 12586 |
| every flat leaf kind, columns added and rows written in the seal's install | seal flip | 2 | 24685 | 23565 | 19520 | 8 | 54446 | 16943 |
| every flat leaf kind, columns added and rows written in the seal's install | seal install | 1298 | 81844 | 65780 | 73680 | 9 | 108872 | 22962 |
| every flat leaf kind, columns added and rows written in the seal's install | seal last | 10 | 71818 | 61424 | 88500 | 8 | 79818 | 13896 |
| every flat leaf kind, columns added and rows written in the seal's install | seal layout | 13 | 67027 | 61269 | 66920 | 4 | 79027 | 8091 |
| every flat leaf kind, columns added and rows written in the seal's install | seal pass1 | 1893 | 79087 | 63867 | 120892 | 8 | 103087 | 20055 |
| every flat leaf kind, columns added and rows written in the seal's install | seal snapshot | 16 | 62590 | 56736 | 119550 | 8 | 70590 | 9348 |
| every flat leaf kind, columns added and rows written in the seal's install | seal write | 3321 | 79857 | 64565 | 218776 | 5 | 93427 | 14337 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction begin | 1 | 23434 | 23434 | 19872 | 8 | 55434 | 14065 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction flip | 1 | 50222 | 50222 | 45240 | 4 | 66222 | 5690 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction install | 3 | 62381 | 57324 | 63488 | 3 | 70381 | 9515 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction last | 359 | 66271 | 63280 | 65480 | 7 | 77753 | 6032 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction layout | 2 | 61456 | 55210 | 60040 | 2 | 69456 | 7839 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction pass1 | 2844 | 71508 | 65330 | 110232 | 6 | 76031 | 10726 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction snapshot | 1 | 18717 | 18717 | 22160 | 7 | 46717 | 6075 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | compaction write | 5692 | 72559 | 65551 | 83780 | 5 | 90773 | 10008 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | drain | 1530 | 53269 | 44923 | 126340 | 7 | 73497 | 11435 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release begin | 6 | 50213 | 44267 | 87840 | 8 | 66213 | 25403 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release flip | 1 | 5751 | 5751 | 8200 | 3 | 17751 | 897 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release install | 2 | 62968 | 59269 | 63800 | 2 | 70968 | 10404 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release last | 202 | 65374 | 62745 | 55240 | 7 | 76613 | 6541 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release layout | 1 | 53669 | 53669 | 52840 | 2 | 61669 | 6983 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release pass1 | 2452 | 71635 | 64610 | 90280 | 2 | 76240 | 11460 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release snapshot | 1 | 11166 | 11166 | 14600 | 3 | 23166 | 2118 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | release write | 4624 | 72392 | 65014 | 72404 | 5 | 92392 | 10588 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal begin | 2 | 21181 | 16977 | 19472 | 8 | 53181 | 12586 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal flip | 2 | 17159 | 13404 | 16640 | 6 | 41159 | 8598 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal install | 7 | 63777 | 55855 | 106952 | 4 | 75393 | 12684 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal last | 45 | 70567 | 62275 | 119320 | 6 | 86800 | 7145 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal layout | 3 | 61729 | 39459 | 60040 | 3 | 73729 | 7865 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal pass1 | 555 | 74825 | 64964 | 124790 | 5 | 90825 | 9444 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal snapshot | 15 | 63048 | 59544 | 122350 | 8 | 71048 | 10729 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | seal write | 543 | 77020 | 68918 | 104330 | 6 | 94751 | 8408 |
| mixed rows, the table opened with a newer schema in its seal | compaction begin | 1 | 25274 | 25274 | 20992 | 8 | 57274 | 14138 |
| mixed rows, the table opened with a newer schema in its seal | compaction flip | 1 | 45829 | 45829 | 40860 | 4 | 61829 | 5740 |
| mixed rows, the table opened with a newer schema in its seal | compaction install | 3 | 64679 | 58810 | 64232 | 3 | 72679 | 10340 |
| mixed rows, the table opened with a newer schema in its seal | compaction last | 359 | 66106 | 64140 | 65560 | 5 | 73880 | 5912 |
| mixed rows, the table opened with a newer schema in its seal | compaction layout | 1 | 62846 | 62846 | 59800 | 3 | 74846 | 7125 |
| mixed rows, the table opened with a newer schema in its seal | compaction pass1 | 2888 | 75571 | 66088 | 139475 | 4 | 87049 | 15173 |
| mixed rows, the table opened with a newer schema in its seal | compaction snapshot | 1 | 45621 | 45621 | 42880 | 4 | 61621 | 5740 |
| mixed rows, the table opened with a newer schema in its seal | compaction write | 7718 | 74503 | 66164 | 142804 | 5 | 91867 | 9712 |
| mixed rows, the table opened with a newer schema in its seal | drain | 472 | 53098 | 41812 | 126340 | 7 | 71123 | 11271 |
| mixed rows, the table opened with a newer schema in its seal | release begin | 11 | 52174 | 45596 | 96640 | 8 | 72291 | 25410 |
| mixed rows, the table opened with a newer schema in its seal | release flip | 1 | 5767 | 5767 | 8200 | 3 | 17767 | 897 |
| mixed rows, the table opened with a newer schema in its seal | release install | 3 | 65695 | 52786 | 64232 | 2 | 73695 | 11015 |
| mixed rows, the table opened with a newer schema in its seal | release last | 304 | 65196 | 62285 | 42640 | 4 | 77768 | 7799 |
| mixed rows, the table opened with a newer schema in its seal | release layout | 1 | 54218 | 54218 | 53000 | 3 | 66218 | 7125 |
| mixed rows, the table opened with a newer schema in its seal | release pass1 | 3300 | 74142 | 65169 | 98090 | 4 | 82142 | 16064 |
| mixed rows, the table opened with a newer schema in its seal | release snapshot | 1 | 11190 | 11190 | 14600 | 3 | 23190 | 2118 |
| mixed rows, the table opened with a newer schema in its seal | release write | 8795 | 71646 | 66024 | 66904 | 5 | 91295 | 12691 |
| mixed rows, the table opened with a newer schema in its seal | seal begin | 2 | 21400 | 17087 | 20592 | 8 | 53400 | 11876 |
| mixed rows, the table opened with a newer schema in its seal | seal flip | 2 | 17159 | 13404 | 16640 | 6 | 41159 | 8598 |
| mixed rows, the table opened with a newer schema in its seal | seal install | 7 | 63777 | 55088 | 106952 | 4 | 71777 | 12801 |
| mixed rows, the table opened with a newer schema in its seal | seal last | 26 | 71163 | 67334 | 130030 | 6 | 81861 | 6948 |
| mixed rows, the table opened with a newer schema in its seal | seal layout | 3 | 61729 | 39529 | 60040 | 3 | 73729 | 7865 |
| mixed rows, the table opened with a newer schema in its seal | seal pass1 | 228 | 74012 | 68952 | 124520 | 4 | 89967 | 8839 |
| mixed rows, the table opened with a newer schema in its seal | seal snapshot | 17 | 66226 | 60979 | 123000 | 4 | 82226 | 10729 |
| mixed rows, the table opened with a newer schema in its seal | seal write | 397 | 77652 | 71524 | 104226 | 5 | 97053 | 9460 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | drain | 58 | 46634 | 19857 | 42440 | 7 | 58634 | 8643 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | drop | 1 | 54182 | 54182 | 47400 | 6 | 78182 | 17709 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal begin | 2 | 13864 | 13674 | 11912 | 7 | 41864 | 865 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal flip | 2 | 8841 | 8841 | 8440 | 5 | 28841 | 900 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal install | 14 | 68540 | 62015 | 87016 | 7 | 92540 | 9633 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal last | 2 | 41614 | 41554 | 35180 | 6 | 65614 | 7067 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal layout | 2 | 49565 | 49153 | 46280 | 5 | 69565 | 8587 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal pass1 | 6150 | 76620 | 64066 | 169080 | 11 | 106730 | 16495 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal snapshot | 12 | 63509 | 60116 | 84300 | 4 | 71509 | 11079 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | seal write | 5909 | 77644 | 65629 | 108230 | 4 | 89644 | 10475 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction begin | 1 | 24313 | 24313 | 20992 | 8 | 56313 | 14065 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction flip | 1 | 50272 | 50272 | 45240 | 4 | 66272 | 5690 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction install | 48 | 63898 | 61941 | 45168 | 6 | 87898 | 9560 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction last | 389 | 67072 | 62213 | 65760 | 4 | 76652 | 6008 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction layout | 1 | 34786 | 34786 | 32120 | 5 | 54786 | 7235 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction pass1 | 26229 | 73246 | 55807 | 88660 | 8 | 97366 | 20287 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction snapshot | 1 | 49958 | 49958 | 47260 | 4 | 65958 | 5740 |
| Nat and Int sums past 65,535 bytes: seal, compaction | compaction write | 6432 | 72558 | 67006 | 66292 | 6 | 90108 | 10968 |
| Nat and Int sums past 65,535 bytes: seal, compaction | drain | 784 | 51224 | 42256 | 52900 | 7 | 71224 | 10400 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal flip | 1 | 9438 | 9438 | 8520 | 5 | 29438 | 900 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal install | 47 | 64259 | 62155 | 87640 | 8 | 95390 | 10548 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal last | 487 | 63837 | 62233 | 62960 | 7 | 91250 | 8106 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal layout | 1 | 33957 | 33957 | 32120 | 5 | 53957 | 7328 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal pass1 | 37113 | 76171 | 53409 | 113490 | 10 | 104440 | 19862 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal snapshot | 9 | 66681 | 57100 | 94900 | 4 | 82681 | 10029 |
| Nat and Int sums past 65,535 bytes: seal, compaction | seal write | 6583 | 67764 | 64732 | 55816 | 6 | 87989 | 11235 |
| 120 Bool and tag leaves: compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| 120 Bool and tag leaves: compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| 120 Bool and tag leaves: compaction, column release | compaction install | 43 | 68811 | 62811 | 62744 | 6 | 89164 | 12136 |
| 120 Bool and tag leaves: compaction, column release | compaction last | 7 | 61499 | 60614 | 64040 | 4 | 72431 | 6541 |
| 120 Bool and tag leaves: compaction, column release | compaction layout | 14 | 66522 | 63321 | 66920 | 3 | 78522 | 7509 |
| 120 Bool and tag leaves: compaction, column release | compaction pass1 | 1537 | 67743 | 62827 | 100560 | 6 | 90590 | 10795 |
| 120 Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 12580 | 12580 | 14600 | 3 | 24580 | 2154 |
| 120 Bool and tag leaves: compaction, column release | compaction write | 1750 | 67827 | 62768 | 72404 | 4 | 82518 | 7568 |
| 120 Bool and tag leaves: compaction, column release | drain | 73 | 47426 | 18618 | 42840 | 7 | 63426 | 8643 |
| 120 Bool and tag leaves: compaction, column release | release begin | 13 | 48705 | 44036 | 87760 | 8 | 63211 | 26731 |
| 120 Bool and tag leaves: compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| 120 Bool and tag leaves: compaction, column release | release install | 41 | 69085 | 62646 | 67024 | 6 | 89643 | 12006 |
| 120 Bool and tag leaves: compaction, column release | release last | 7 | 61612 | 59035 | 64360 | 4 | 65612 | 6541 |
| 120 Bool and tag leaves: compaction, column release | release layout | 14 | 66648 | 61541 | 66920 | 3 | 78648 | 7509 |
| 120 Bool and tag leaves: compaction, column release | release pass1 | 1584 | 68769 | 65971 | 90280 | 6 | 92769 | 10795 |
| 120 Bool and tag leaves: compaction, column release | release snapshot | 1 | 12706 | 12706 | 14600 | 3 | 24706 | 2154 |
| 120 Bool and tag leaves: compaction, column release | release write | 1809 | 68888 | 65360 | 66904 | 4 | 79197 | 8257 |
| 120 Bool and tag leaves: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 120 Bool and tag leaves: compaction, column release | seal flip | 1 | 13839 | 13839 | 12040 | 5 | 33839 | 900 |
| 120 Bool and tag leaves: compaction, column release | seal install | 35 | 63738 | 61477 | 77000 | 7 | 90273 | 6405 |
| 120 Bool and tag leaves: compaction, column release | seal last | 7 | 63266 | 59734 | 67320 | 5 | 69843 | 6948 |
| 120 Bool and tag leaves: compaction, column release | seal layout | 16 | 61242 | 57525 | 60440 | 3 | 73242 | 7725 |
| 120 Bool and tag leaves: compaction, column release | seal pass1 | 641 | 74778 | 71854 | 99460 | 8 | 97452 | 18889 |
| 120 Bool and tag leaves: compaction, column release | seal snapshot | 8 | 61949 | 61889 | 111950 | 4 | 77519 | 9529 |
| 120 Bool and tag leaves: compaction, column release | seal write | 868 | 74669 | 71238 | 90276 | 4 | 90573 | 5799 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 18467 | 18467 | 17712 | 8 | 50467 | 13406 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction install | 31 | 66291 | 64511 | 73448 | 6 | 87586 | 10276 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction last | 420 | 64947 | 62333 | 86880 | 4 | 72470 | 3960 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 22 | 67614 | 61387 | 66520 | 3 | 79614 | 7770 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 5262 | 71656 | 65086 | 118971 | 6 | 91184 | 11161 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 21946 | 21946 | 23280 | 7 | 49946 | 7004 |
| 170 Text running records, Bool and tag leaves: compaction, column release | compaction write | 26394 | 71827 | 64711 | 133780 | 7 | 97426 | 10443 |
| 170 Text running records, Bool and tag leaves: compaction, column release | drain | 1302 | 55861 | 46428 | 62480 | 7 | 77739 | 16412 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release begin | 16 | 49968 | 44433 | 87760 | 8 | 63301 | 26821 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release install | 30 | 65466 | 63620 | 73768 | 6 | 86972 | 10729 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release last | 549 | 64909 | 62916 | 63840 | 4 | 79061 | 6541 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release layout | 22 | 67181 | 59353 | 66840 | 3 | 79181 | 7711 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 8420 | 71066 | 64533 | 99692 | 6 | 93984 | 11256 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 13466 | 13466 | 14600 | 3 | 25466 | 2370 |
| 170 Text running records, Bool and tag leaves: compaction, column release | release write | 46620 | 72611 | 65436 | 92880 | 6 | 96611 | 9997 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 13898 | 13898 | 11912 | 7 | 41898 | 865 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 37557 | 37557 | 22040 | 5 | 57557 | 900 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal install | 50 | 78566 | 62635 | 63992 | 10 | 109915 | 19639 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal last | 128 | 71664 | 68369 | 125810 | 5 | 87851 | 7145 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal layout | 22 | 61804 | 60333 | 60440 | 3 | 73804 | 7720 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 1191 | 74135 | 67891 | 118450 | 6 | 97817 | 20672 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 1 | 51954 | 51954 | 62700 | 4 | 67954 | 10779 |
| 170 Text running records, Bool and tag leaves: compaction, column release | seal write | 3315 | 81266 | 70231 | 96568 | 7 | 105266 | 9843 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 16458 | 16458 | 12312 | 7 | 44458 | 14330 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction install | 18 | 65661 | 63450 | 73448 | 4 | 81570 | 10643 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction last | 650 | 65059 | 62445 | 84080 | 4 | 73378 | 3905 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 12 | 67387 | 61651 | 66520 | 3 | 75436 | 7727 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 5001 | 71656 | 64938 | 118365 | 6 | 90617 | 11463 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 20826 | 20826 | 23280 | 7 | 48826 | 7004 |
| 90 Text running records, Bool and tag leaves: compaction, column release | compaction write | 25215 | 71948 | 64722 | 127104 | 6 | 95948 | 10363 |
| 90 Text running records, Bool and tag leaves: compaction, column release | drain | 1058 | 59183 | 44936 | 62480 | 7 | 80246 | 16436 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release begin | 11 | 49968 | 44996 | 87760 | 8 | 63301 | 26821 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release install | 18 | 64864 | 59939 | 73768 | 4 | 80864 | 9762 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release last | 761 | 66822 | 62835 | 59920 | 5 | 78822 | 6541 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release layout | 12 | 66981 | 58414 | 66840 | 3 | 74981 | 7727 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 7653 | 71243 | 64528 | 99692 | 6 | 91661 | 11139 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 12346 | 12346 | 14600 | 3 | 24346 | 2118 |
| 90 Text running records, Bool and tag leaves: compaction, column release | release write | 42593 | 72611 | 65535 | 92880 | 6 | 96611 | 9962 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 22786 | 22786 | 15800 | 5 | 42786 | 900 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal install | 27 | 77025 | 63427 | 70392 | 6 | 97025 | 18503 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal last | 198 | 72088 | 68740 | 110340 | 6 | 83874 | 6948 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal layout | 12 | 62435 | 60656 | 60440 | 3 | 74435 | 7776 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 1169 | 75096 | 68196 | 104060 | 8 | 105474 | 20771 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 8 | 61659 | 58406 | 114950 | 4 | 69659 | 9329 |
| 90 Text running records, Bool and tag leaves: compaction, column release | seal write | 3206 | 80304 | 70995 | 84972 | 7 | 104304 | 9843 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction begin | 1 | 22945 | 22945 | 20992 | 8 | 54945 | 13406 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction install | 13 | 66434 | 62254 | 73448 | 4 | 82434 | 10643 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction last | 775 | 65046 | 62383 | 84180 | 5 | 71659 | 3874 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction layout | 9 | 67387 | 61260 | 66520 | 3 | 75387 | 7855 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction pass1 | 4660 | 71824 | 64945 | 117943 | 6 | 88501 | 12266 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction snapshot | 1 | 45388 | 45388 | 42880 | 4 | 61388 | 5740 |
| 60 Text running records, Bool and tag leaves: compaction, column release | compaction write | 23267 | 73264 | 64752 | 127504 | 6 | 97264 | 10351 |
| 60 Text running records, Bool and tag leaves: compaction, column release | drain | 873 | 59604 | 44472 | 62480 | 7 | 76267 | 16436 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release begin | 9 | 49940 | 44168 | 87760 | 8 | 65940 | 26821 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release install | 12 | 62607 | 62039 | 73768 | 4 | 78470 | 9714 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release last | 827 | 65182 | 62942 | 59500 | 4 | 75699 | 6541 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release layout | 8 | 66981 | 58589 | 66840 | 3 | 74981 | 7855 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release pass1 | 6791 | 71201 | 64550 | 100092 | 6 | 90098 | 12504 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release snapshot | 1 | 11922 | 11922 | 14600 | 3 | 23922 | 2118 |
| 60 Text running records, Bool and tag leaves: compaction, column release | release write | 37390 | 72611 | 65478 | 92880 | 6 | 96611 | 9997 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal flip | 1 | 19276 | 19276 | 13400 | 5 | 39276 | 900 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal install | 19 | 76861 | 62592 | 75192 | 7 | 104861 | 19102 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal last | 190 | 72053 | 69022 | 114140 | 6 | 80053 | 6948 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal layout | 9 | 61564 | 55646 | 60440 | 3 | 73564 | 7771 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal pass1 | 1011 | 74474 | 68032 | 103231 | 6 | 96499 | 19124 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal snapshot | 8 | 62154 | 59778 | 113800 | 4 | 70154 | 9429 |
| 60 Text running records, Bool and tag leaves: compaction, column release | seal write | 2653 | 80495 | 70806 | 89868 | 7 | 104495 | 9843 |
| a 200-case tag: a drop | drain | 24 | 12754 | 10098 | 14840 | 7 | 38171 | 7396 |
| a 200-case tag: a drop | drop | 26 | 65817 | 64656 | 64200 | 5 | 73516 | 20256 |
| a 200-case tag: a drop | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 200-case tag: a drop | seal flip | 1 | 8930 | 8930 | 8440 | 5 | 28930 | 900 |
| a 200-case tag: a drop | seal install | 2 | 62285 | 47299 | 90536 | 4 | 70285 | 4794 |
| a 200-case tag: a drop | seal last | 1 | 30981 | 30981 | 25340 | 6 | 54981 | 6948 |
| a 200-case tag: a drop | seal layout | 1 | 26268 | 26268 | 25240 | 3 | 38268 | 7113 |
| a 200-case tag: a drop | seal pass1 | 535 | 73055 | 64491 | 124570 | 5 | 83711 | 8163 |
| a 200-case tag: a drop | seal snapshot | 7 | 60874 | 60623 | 86600 | 4 | 75122 | 10779 |
| a 200-case tag: a drop | seal write | 692 | 76094 | 64584 | 99866 | 4 | 88094 | 7811 |
| a 200-case tag, every row updated: a drop | drain | 24 | 12754 | 10099 | 14840 | 7 | 38171 | 7396 |
| a 200-case tag, every row updated: a drop | drop | 26 | 67593 | 66580 | 64200 | 5 | 74182 | 26630 |
| a 200-case tag, every row updated: a drop | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 200-case tag, every row updated: a drop | seal flip | 1 | 8930 | 8930 | 8440 | 5 | 28930 | 900 |
| a 200-case tag, every row updated: a drop | seal install | 2 | 62285 | 47299 | 90536 | 4 | 70285 | 4794 |
| a 200-case tag, every row updated: a drop | seal last | 1 | 30981 | 30981 | 25340 | 6 | 54981 | 6948 |
| a 200-case tag, every row updated: a drop | seal layout | 1 | 26268 | 26268 | 25240 | 3 | 38268 | 7113 |
| a 200-case tag, every row updated: a drop | seal pass1 | 535 | 73055 | 64491 | 124570 | 5 | 83711 | 8163 |
| a 200-case tag, every row updated: a drop | seal snapshot | 7 | 60874 | 60623 | 86600 | 4 | 75122 | 10779 |
| a 200-case tag, every row updated: a drop | seal write | 692 | 76094 | 64584 | 99866 | 4 | 88094 | 7811 |
| a 200-case tag, a third of the rows updated: a drop | drain | 24 | 12754 | 10098 | 14840 | 7 | 38171 | 7396 |
| a 200-case tag, a third of the rows updated: a drop | drop | 26 | 67575 | 66415 | 64200 | 5 | 74626 | 25878 |
| a 200-case tag, a third of the rows updated: a drop | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 200-case tag, a third of the rows updated: a drop | seal flip | 1 | 8930 | 8930 | 8440 | 5 | 28930 | 900 |
| a 200-case tag, a third of the rows updated: a drop | seal install | 2 | 62285 | 47299 | 90536 | 4 | 70285 | 4794 |
| a 200-case tag, a third of the rows updated: a drop | seal last | 1 | 30981 | 30981 | 25340 | 6 | 54981 | 6948 |
| a 200-case tag, a third of the rows updated: a drop | seal layout | 1 | 26268 | 26268 | 25240 | 3 | 38268 | 7113 |
| a 200-case tag, a third of the rows updated: a drop | seal pass1 | 535 | 73055 | 64491 | 124570 | 5 | 83711 | 8163 |
| a 200-case tag, a third of the rows updated: a drop | seal snapshot | 7 | 60874 | 60623 | 86600 | 4 | 75122 | 10779 |
| a 200-case tag, a third of the rows updated: a drop | seal write | 692 | 76094 | 64584 | 99866 | 4 | 88094 | 7811 |
| a 200-case tag: compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 200-case tag: compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 200-case tag: compaction, column release | compaction install | 28 | 67649 | 63637 | 84960 | 2 | 75649 | 10267 |
| a 200-case tag: compaction, column release | compaction last | 1 | 33008 | 33008 | 27240 | 6 | 57008 | 6541 |
| a 200-case tag: compaction, column release | compaction layout | 1 | 25866 | 25866 | 25240 | 2 | 33866 | 6951 |
| a 200-case tag: compaction, column release | compaction pass1 | 1078 | 67691 | 64499 | 84560 | 4 | 82427 | 8282 |
| a 200-case tag: compaction, column release | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 200-case tag: compaction, column release | compaction write | 794 | 69930 | 65965 | 72801 | 4 | 83309 | 8931 |
| a 200-case tag: compaction, column release | drain | 24 | 12754 | 9992 | 14840 | 7 | 38171 | 7396 |
| a 200-case tag: compaction, column release | release begin | 6 | 33812 | 30507 | 77000 | 8 | 62547 | 26067 |
| a 200-case tag: compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 200-case tag: compaction, column release | release install | 28 | 67775 | 63491 | 84960 | 2 | 75775 | 10267 |
| a 200-case tag: compaction, column release | release last | 1 | 30344 | 30344 | 24040 | 6 | 54344 | 6541 |
| a 200-case tag: compaction, column release | release layout | 1 | 19222 | 19222 | 18440 | 2 | 27222 | 6951 |
| a 200-case tag: compaction, column release | release pass1 | 881 | 65986 | 62998 | 76060 | 4 | 80133 | 8282 |
| a 200-case tag: compaction, column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 200-case tag: compaction, column release | release write | 572 | 70502 | 65147 | 64098 | 4 | 82170 | 9620 |
| a 200-case tag: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 200-case tag: compaction, column release | seal flip | 1 | 8930 | 8930 | 8440 | 5 | 28930 | 900 |
| a 200-case tag: compaction, column release | seal install | 2 | 62285 | 47299 | 90536 | 4 | 70285 | 4794 |
| a 200-case tag: compaction, column release | seal last | 1 | 30981 | 30981 | 25340 | 6 | 54981 | 6948 |
| a 200-case tag: compaction, column release | seal layout | 1 | 26268 | 26268 | 25240 | 3 | 38268 | 7113 |
| a 200-case tag: compaction, column release | seal pass1 | 535 | 73055 | 64491 | 124570 | 5 | 83711 | 8163 |
| a 200-case tag: compaction, column release | seal snapshot | 7 | 60874 | 60623 | 86600 | 4 | 75122 | 10779 |
| a 200-case tag: compaction, column release | seal write | 692 | 76094 | 64584 | 99866 | 4 | 88094 | 7811 |
| a 40-case tag: compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 40-case tag: compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 40-case tag: compaction, column release | compaction install | 7 | 67418 | 56015 | 71320 | 2 | 75418 | 10267 |
| a 40-case tag: compaction, column release | compaction last | 1 | 34145 | 34145 | 26940 | 8 | 66145 | 6541 |
| a 40-case tag: compaction, column release | compaction layout | 1 | 25866 | 25866 | 25240 | 2 | 33866 | 6951 |
| a 40-case tag: compaction, column release | compaction pass1 | 1077 | 67691 | 63863 | 84560 | 2 | 73148 | 7448 |
| a 40-case tag: compaction, column release | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 40-case tag: compaction, column release | compaction write | 739 | 69188 | 66007 | 72644 | 4 | 80847 | 8177 |
| a 40-case tag: compaction, column release | drain | 14 | 10238 | 10046 | 12000 | 7 | 38238 | 7463 |
| a 40-case tag: compaction, column release | release begin | 6 | 33812 | 29771 | 77000 | 8 | 62547 | 26067 |
| a 40-case tag: compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 40-case tag: compaction, column release | release install | 7 | 63577 | 55961 | 71320 | 2 | 71577 | 10267 |
| a 40-case tag: compaction, column release | release last | 1 | 30585 | 30585 | 23740 | 8 | 62585 | 6541 |
| a 40-case tag: compaction, column release | release layout | 1 | 19222 | 19222 | 18440 | 2 | 27222 | 6951 |
| a 40-case tag: compaction, column release | release pass1 | 847 | 68337 | 64212 | 76060 | 2 | 72337 | 8137 |
| a 40-case tag: compaction, column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 40-case tag: compaction, column release | release write | 514 | 68207 | 65371 | 67201 | 4 | 82607 | 8866 |
| a 40-case tag: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 40-case tag: compaction, column release | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 40-case tag: compaction, column release | seal install | 2 | 60854 | 42151 | 94944 | 2 | 68854 | 4785 |
| a 40-case tag: compaction, column release | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 40-case tag: compaction, column release | seal layout | 1 | 25992 | 25992 | 25240 | 2 | 33992 | 7005 |
| a 40-case tag: compaction, column release | seal pass1 | 504 | 72617 | 65649 | 124190 | 2 | 80617 | 6142 |
| a 40-case tag: compaction, column release | seal snapshot | 6 | 60804 | 59320 | 80000 | 4 | 68804 | 9729 |
| a 40-case tag: compaction, column release | seal write | 602 | 75730 | 65442 | 99810 | 4 | 87730 | 7408 |
| a 17-case tag: compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 17-case tag: compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 17-case tag: compaction, column release | compaction install | 3 | 66099 | 61315 | 68760 | 2 | 74099 | 10267 |
| a 17-case tag: compaction, column release | compaction last | 1 | 34128 | 34128 | 26940 | 8 | 66128 | 6541 |
| a 17-case tag: compaction, column release | compaction layout | 1 | 25866 | 25866 | 25240 | 2 | 33866 | 6951 |
| a 17-case tag: compaction, column release | compaction pass1 | 1077 | 67691 | 63654 | 84560 | 2 | 72546 | 7448 |
| a 17-case tag: compaction, column release | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 17-case tag: compaction, column release | compaction write | 794 | 68344 | 65449 | 72644 | 4 | 84344 | 7797 |
| a 17-case tag: compaction, column release | drain | 13 | 10238 | 10114 | 12000 | 7 | 38238 | 7463 |
| a 17-case tag: compaction, column release | release begin | 6 | 33812 | 29771 | 77000 | 8 | 62547 | 26067 |
| a 17-case tag: compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 17-case tag: compaction, column release | release install | 3 | 66225 | 58931 | 68760 | 2 | 74225 | 10267 |
| a 17-case tag: compaction, column release | release last | 1 | 30585 | 30585 | 23740 | 8 | 62585 | 6541 |
| a 17-case tag: compaction, column release | release layout | 1 | 19222 | 19222 | 18440 | 2 | 27222 | 6951 |
| a 17-case tag: compaction, column release | release pass1 | 824 | 68037 | 65414 | 76060 | 2 | 72037 | 8137 |
| a 17-case tag: compaction, column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 17-case tag: compaction, column release | release write | 587 | 69282 | 64123 | 56898 | 4 | 81777 | 8843 |
| a 17-case tag: compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 17-case tag: compaction, column release | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 17-case tag: compaction, column release | seal install | 2 | 61317 | 41296 | 100392 | 2 | 69317 | 4785 |
| a 17-case tag: compaction, column release | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 17-case tag: compaction, column release | seal layout | 1 | 25992 | 25992 | 25240 | 2 | 33992 | 7005 |
| a 17-case tag: compaction, column release | seal pass1 | 72 | 74876 | 69934 | 124040 | 2 | 80555 | 5923 |
| a 17-case tag: compaction, column release | seal snapshot | 6 | 62004 | 57733 | 81800 | 4 | 70004 | 12479 |
| a 17-case tag: compaction, column release | seal write | 50 | 74704 | 73029 | 98688 | 4 | 86704 | 9019 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction begin | 2 | 18688 | 18541 | 17712 | 7 | 46688 | 14065 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction flip | 2 | 11380 | 11380 | 13840 | 6 | 35380 | 4620 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction install | 41 | 64798 | 61320 | 45168 | 8 | 95108 | 12124 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction last | 2 | 36608 | 33332 | 28620 | 6 | 60608 | 11403 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction layout | 2 | 33719 | 33719 | 28520 | 5 | 53719 | 7172 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction pass1 | 24112 | 77806 | 68468 | 84960 | 10 | 108058 | 20859 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction snapshot | 2 | 16585 | 15959 | 20240 | 6 | 40585 | 4977 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | compaction write | 6972 | 75585 | 67706 | 60128 | 8 | 101029 | 14709 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | drain | 570 | 54406 | 27537 | 50500 | 7 | 73772 | 12296 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal flip | 1 | 9293 | 9293 | 8520 | 5 | 29293 | 900 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal install | 11758 | 75915 | 68514 | 52464 | 8 | 95859 | 22751 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal last | 1 | 36739 | 36739 | 21320 | 6 | 60739 | 11488 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal layout | 1 | 32685 | 32685 | 28520 | 5 | 52685 | 7172 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal pass1 | 11766 | 74913 | 66240 | 107170 | 9 | 102913 | 19829 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal snapshot | 3 | 61069 | 59569 | 85150 | 4 | 72621 | 9379 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | seal write | 2845 | 70336 | 65419 | 60128 | 7 | 98070 | 14426 |
| narrow rows, a Nat key and two Nats: seal | drain | 20 | 10238 | 10083 | 12000 | 7 | 38238 | 7463 |
| narrow rows, a Nat key and two Nats: seal | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| narrow rows, a Nat key and two Nats: seal | seal flip | 1 | 9039 | 9039 | 8440 | 5 | 29039 | 900 |
| narrow rows, a Nat key and two Nats: seal | seal install | 2 | 60690 | 41624 | 99984 | 2 | 68690 | 4785 |
| narrow rows, a Nat key and two Nats: seal | seal last | 1 | 30098 | 30098 | 25340 | 6 | 54098 | 6948 |
| narrow rows, a Nat key and two Nats: seal | seal layout | 1 | 25929 | 25929 | 25240 | 2 | 33929 | 6944 |
| narrow rows, a Nat key and two Nats: seal | seal pass1 | 87 | 72864 | 69981 | 118390 | 2 | 80864 | 5833 |
| narrow rows, a Nat key and two Nats: seal | seal snapshot | 7 | 64124 | 58330 | 89800 | 4 | 72124 | 10379 |
| narrow rows, a Nat key and two Nats: seal | seal write | 37 | 75288 | 73656 | 86032 | 4 | 89521 | 5567 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction install | 5 | 64512 | 60337 | 115640 | 2 | 72512 | 11130 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction last | 1 | 34137 | 34137 | 26940 | 8 | 66137 | 6541 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction layout | 1 | 24948 | 24948 | 21640 | 2 | 32948 | 6951 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction pass1 | 988 | 67691 | 63419 | 84560 | 2 | 74722 | 7448 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 17-case tag, 64 deletes after a compaction's snapshot | compaction write | 457 | 68344 | 65324 | 56916 | 4 | 84344 | 7832 |
| a 17-case tag, 64 deletes after a compaction's snapshot | drain | 13 | 10238 | 10114 | 12000 | 7 | 38238 | 7463 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal install | 2 | 61317 | 41296 | 100392 | 2 | 69317 | 4785 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal layout | 1 | 25020 | 25020 | 21640 | 2 | 33020 | 7005 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal pass1 | 74 | 73739 | 70672 | 118240 | 2 | 80765 | 5657 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal snapshot | 6 | 62004 | 57733 | 81800 | 4 | 70004 | 12479 |
| a 17-case tag, 64 deletes after a compaction's snapshot | seal write | 35 | 74675 | 71343 | 62417 | 4 | 86461 | 9019 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction install | 4 | 63994 | 50769 | 148680 | 2 | 71994 | 10267 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction last | 1 | 35428 | 35428 | 26940 | 8 | 67428 | 6541 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction layout | 1 | 24948 | 24948 | 21640 | 2 | 32948 | 6951 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction pass1 | 988 | 67691 | 63419 | 84560 | 2 | 74722 | 7448 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | compaction write | 457 | 68344 | 65324 | 56916 | 4 | 84344 | 7832 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | drain | 13 | 10238 | 10114 | 12000 | 7 | 38238 | 7463 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal install | 2 | 61317 | 41296 | 100392 | 2 | 69317 | 4785 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal layout | 1 | 25020 | 25020 | 21640 | 2 | 33020 | 7005 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal pass1 | 74 | 73739 | 70672 | 118240 | 2 | 80765 | 5657 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal snapshot | 6 | 62004 | 57733 | 81800 | 4 | 70004 | 12479 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | seal write | 35 | 74675 | 71343 | 62417 | 4 | 86461 | 9019 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction install | 4 | 67736 | 62188 | 95088 | 2 | 75736 | 12504 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction last | 1 | 35428 | 35428 | 26940 | 8 | 67428 | 6541 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction layout | 1 | 24948 | 24948 | 21640 | 2 | 32948 | 6951 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction pass1 | 988 | 67691 | 63435 | 84560 | 2 | 74722 | 7448 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | compaction write | 392 | 67931 | 66361 | 61619 | 4 | 80897 | 7458 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | drain | 13 | 10238 | 10114 | 12000 | 7 | 38238 | 7463 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal install | 2 | 61173 | 41224 | 100176 | 2 | 69173 | 4785 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal layout | 1 | 25020 | 25020 | 21640 | 2 | 33020 | 7005 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal pass1 | 74 | 74426 | 70857 | 118240 | 2 | 80555 | 5570 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal snapshot | 6 | 62004 | 57733 | 81800 | 4 | 70004 | 12479 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | seal write | 29 | 74510 | 72264 | 72865 | 4 | 86510 | 5788 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction install | 14 | 66060 | 59808 | 163080 | 4 | 77794 | 13756 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction last | 1 | 35428 | 35428 | 26940 | 8 | 67428 | 6541 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction layout | 1 | 24948 | 24948 | 21640 | 2 | 32948 | 6951 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction pass1 | 988 | 67691 | 63731 | 84560 | 2 | 75270 | 7448 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | compaction write | 549 | 74089 | 65537 | 51413 | 4 | 90089 | 13017 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | drain | 15 | 10238 | 9989 | 12000 | 7 | 38238 | 7463 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal begin | 1 | 13898 | 13898 | 11912 | 7 | 41898 | 865 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal flip | 1 | 8890 | 8890 | 8440 | 5 | 28890 | 900 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal install | 2 | 60726 | 42087 | 94752 | 2 | 68726 | 4785 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal last | 1 | 30107 | 30107 | 25340 | 6 | 54107 | 6948 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal layout | 1 | 25020 | 25020 | 21640 | 2 | 33020 | 7005 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal pass1 | 507 | 72836 | 66002 | 118390 | 2 | 80836 | 6184 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal snapshot | 6 | 62859 | 60390 | 83650 | 4 | 70859 | 11729 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | seal write | 822 | 70197 | 63564 | 37695 | 4 | 86197 | 13055 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction install | 55 | 69594 | 62373 | 44000 | 6 | 84882 | 16403 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction last | 1 | 34387 | 34387 | 27240 | 6 | 58387 | 6541 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction layout | 1 | 24948 | 24948 | 21640 | 2 | 32948 | 6951 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction pass1 | 989 | 67691 | 64144 | 101720 | 4 | 79014 | 8157 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | compaction write | 457 | 66287 | 63895 | 61716 | 4 | 82287 | 8580 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | drain | 15 | 12754 | 9819 | 14840 | 7 | 38171 | 7396 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal flip | 1 | 8862 | 8862 | 8360 | 5 | 28862 | 900 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal install | 2 | 60939 | 43345 | 89104 | 3 | 68939 | 4794 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal last | 1 | 30981 | 30981 | 25340 | 6 | 54981 | 6948 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal layout | 1 | 25296 | 25296 | 21640 | 3 | 37296 | 7113 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal pass1 | 512 | 73187 | 66656 | 118240 | 4 | 85187 | 8038 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | seal write | 659 | 69406 | 64367 | 54101 | 4 | 82803 | 7761 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction install | 6 | 68051 | 56417 | 49600 | 6 | 92051 | 14589 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction last | 1 | 35732 | 35732 | 27240 | 6 | 59732 | 6541 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction layout | 1 | 25376 | 25376 | 21640 | 3 | 37376 | 7135 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction pass1 | 1638 | 71438 | 65750 | 177080 | 7 | 98059 | 17108 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | compaction write | 550 | 69087 | 65968 | 59410 | 5 | 85331 | 10664 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | drain | 35 | 46634 | 26607 | 42440 | 7 | 58634 | 8024 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 101093 | 16870 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction install | 119 | 71193 | 63641 | 53320 | 8 | 94631 | 17267 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction last | 1 | 35732 | 35732 | 27240 | 6 | 59732 | 6541 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction layout | 1 | 25376 | 25376 | 21640 | 3 | 37376 | 7135 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction pass1 | 1638 | 71438 | 65750 | 177080 | 7 | 98059 | 17108 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | compaction write | 550 | 69087 | 65968 | 59410 | 5 | 85331 | 10664 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | drain | 35 | 46634 | 26604 | 42440 | 7 | 58634 | 8024 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 101093 | 16870 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction begin | 2 | 18688 | 16848 | 17712 | 7 | 46688 | 13406 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction flip | 2 | 11421 | 8504 | 13840 | 6 | 35421 | 4661 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction install | 714 | 72453 | 67949 | 50328 | 6 | 92736 | 18860 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction last | 2 | 35221 | 32485 | 27240 | 7 | 63221 | 13372 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction layout | 2 | 30969 | 28172 | 27280 | 6 | 54969 | 7135 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction pass1 | 3771 | 73538 | 65627 | 177080 | 11 | 102146 | 17108 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction snapshot | 2 | 15330 | 13159 | 20000 | 6 | 39330 | 4977 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | compaction write | 1127 | 73872 | 66495 | 81316 | 6 | 86255 | 12991 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | drain | 73 | 53044 | 29122 | 62480 | 7 | 65044 | 9894 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 101093 | 16870 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction begin | 2 | 19347 | 17177 | 17712 | 7 | 47347 | 14065 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction flip | 2 | 11421 | 8504 | 13840 | 6 | 35421 | 4661 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction install | 909 | 74273 | 69036 | 49320 | 9 | 97112 | 19048 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction last | 2 | 39362 | 34689 | 27240 | 6 | 63362 | 12214 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction layout | 2 | 29443 | 27409 | 25140 | 5 | 49443 | 7841 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction pass1 | 3766 | 74640 | 65755 | 177080 | 11 | 114766 | 17108 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction snapshot | 2 | 15330 | 13159 | 20000 | 6 | 39330 | 4977 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | compaction write | 1131 | 73958 | 66541 | 81316 | 5 | 87590 | 13077 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | drain | 83 | 53098 | 28695 | 62480 | 7 | 65150 | 9894 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 101093 | 16870 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction begin | 1 | 20258 | 20258 | 17712 | 7 | 48258 | 14138 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction flip | 1 | 12386 | 12386 | 13840 | 6 | 36386 | 4734 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction install | 5 | 64814 | 59666 | 45168 | 6 | 87050 | 9322 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction last | 1 | 27778 | 27778 | 24040 | 6 | 51778 | 5354 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction layout | 1 | 24668 | 24668 | 23840 | 6 | 48668 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction pass1 | 2587 | 75522 | 67488 | 177080 | 10 | 106789 | 16870 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction snapshot | 1 | 16295 | 16295 | 20000 | 6 | 40295 | 5050 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | compaction write | 828 | 74628 | 70001 | 81316 | 5 | 88296 | 12078 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | drain | 71 | 53044 | 29279 | 62480 | 7 | 65044 | 9894 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release begin | 6 | 49421 | 40487 | 84560 | 8 | 66058 | 26067 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release install | 710 | 72579 | 67979 | 47016 | 8 | 95287 | 18860 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release last | 1 | 37442 | 37442 | 22140 | 6 | 61442 | 12313 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release layout | 1 | 19487 | 19487 | 18440 | 3 | 31487 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release pass1 | 2015 | 73343 | 65786 | 177080 | 7 | 90776 | 17108 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | release write | 822 | 69266 | 65852 | 59410 | 5 | 82879 | 10498 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 101093 | 16870 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction begin | 1 | 18688 | 18688 | 17712 | 7 | 46688 | 13406 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction flip | 1 | 10170 | 10170 | 13600 | 6 | 34170 | 4661 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction install | 5 | 63896 | 60478 | 45168 | 6 | 86132 | 9441 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction last | 1 | 29749 | 29749 | 27240 | 6 | 53749 | 5281 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction layout | 1 | 29772 | 29772 | 27040 | 6 | 53772 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction pass1 | 3043 | 74604 | 66135 | 177080 | 9 | 105844 | 16870 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction snapshot | 1 | 15330 | 15330 | 20000 | 6 | 39330 | 4977 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | compaction write | 828 | 73737 | 69134 | 81316 | 5 | 87708 | 12078 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | drain | 55 | 53098 | 28497 | 62480 | 7 | 65109 | 9070 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal install | 1895 | 82329 | 67857 | 64928 | 9 | 111478 | 22256 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal last | 1 | 34743 | 34743 | 18040 | 6 | 58743 | 12568 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal pass1 | 1530 | 72705 | 64962 | 181080 | 7 | 95006 | 16870 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | drain | 21 | 45666 | 19125 | 42040 | 7 | 49666 | 8024 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal install | 399 | 82329 | 68564 | 64928 | 10 | 114079 | 21289 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal last | 1 | 33226 | 33226 | 25340 | 6 | 57226 | 6948 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal pass1 | 1530 | 72705 | 64963 | 181080 | 9 | 99561 | 16870 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction begin | 3 | 23604 | 19638 | 20992 | 8 | 55604 | 14496 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction flip | 3 | 50206 | 23979 | 45240 | 6 | 66206 | 5690 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction install | 512 | 73885 | 70225 | 47704 | 8 | 95595 | 19063 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction last | 1505 | 65603 | 63076 | 60080 | 5 | 81147 | 11384 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction layout | 3 | 40156 | 36014 | 36400 | 5 | 60156 | 7284 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction pass1 | 228123 | 79418 | 53054 | 84960 | 9 | 101381 | 20168 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction snapshot | 3 | 49958 | 26875 | 47260 | 6 | 65958 | 5740 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | compaction write | 21458 | 71505 | 66481 | 59604 | 6 | 90905 | 10968 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | drain | 2136 | 53091 | 42751 | 54860 | 7 | 73091 | 14956 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release begin | 6 | 42058 | 36752 | 84560 | 8 | 66058 | 26088 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release flip | 1 | 5751 | 5751 | 8200 | 3 | 17751 | 897 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release install | 25 | 63488 | 59186 | 45168 | 6 | 87488 | 9560 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release last | 200 | 65205 | 63024 | 50560 | 5 | 70849 | 8243 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release layout | 1 | 25687 | 25687 | 21720 | 4 | 41687 | 7167 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release pass1 | 19948 | 69439 | 50429 | 76060 | 8 | 98798 | 20287 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | release write | 2802 | 70077 | 66072 | 55816 | 6 | 88239 | 10802 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal flip | 1 | 9372 | 9372 | 8520 | 5 | 29372 | 900 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal install | 6317 | 64526 | 49392 | 64928 | 9 | 96314 | 23080 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal last | 489 | 63616 | 61937 | 62760 | 6 | 78674 | 11455 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal layout | 1 | 32854 | 32854 | 28520 | 4 | 48854 | 7284 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal pass1 | 74952 | 74478 | 50987 | 118540 | 9 | 98982 | 21054 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal snapshot | 7 | 63829 | 61040 | 91150 | 4 | 71829 | 10629 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | seal write | 4996 | 65919 | 64002 | 55816 | 6 | 87217 | 10968 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction begin | 3 | 24130 | 20404 | 21392 | 8 | 56130 | 14065 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction flip | 3 | 50272 | 24440 | 45240 | 6 | 66272 | 5690 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction install | 802 | 73915 | 69873 | 45168 | 8 | 95317 | 19063 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction last | 1495 | 65134 | 63119 | 59880 | 6 | 76493 | 11400 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction layout | 3 | 33943 | 33905 | 28520 | 5 | 53943 | 7284 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction pass1 | 221669 | 79417 | 52223 | 85985 | 10 | 106218 | 21801 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction snapshot | 3 | 49958 | 27292 | 47260 | 6 | 65958 | 5740 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | compaction write | 22202 | 72031 | 66271 | 59592 | 6 | 90110 | 11235 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | drain | 1670 | 53078 | 39232 | 52900 | 7 | 73532 | 10509 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release begin | 6 | 42058 | 36758 | 84560 | 8 | 66058 | 26118 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release flip | 1 | 5767 | 5767 | 8200 | 3 | 17767 | 897 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release install | 36 | 63785 | 61151 | 45168 | 6 | 87785 | 9560 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release last | 200 | 65250 | 63049 | 50560 | 5 | 70888 | 8243 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release layout | 1 | 25687 | 25687 | 21720 | 4 | 41687 | 7167 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release pass1 | 26959 | 69182 | 49117 | 76060 | 8 | 96568 | 20100 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | release write | 2810 | 71231 | 67084 | 55816 | 6 | 88291 | 10802 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal flip | 1 | 9438 | 9438 | 8520 | 5 | 29438 | 900 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal install | 9614 | 71039 | 49735 | 78808 | 8 | 96973 | 22792 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal last | 486 | 63522 | 62084 | 62760 | 8 | 94184 | 11655 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal layout | 1 | 32909 | 32909 | 28520 | 5 | 52909 | 7284 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal pass1 | 65609 | 76133 | 51858 | 118540 | 10 | 104440 | 19862 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal snapshot | 7 | 63879 | 61104 | 91200 | 4 | 71879 | 10579 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | seal write | 5001 | 66314 | 64512 | 55816 | 6 | 88005 | 11235 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction begin | 1 | 22945 | 22945 | 20992 | 8 | 54945 | 13406 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction flip | 1 | 50127 | 50127 | 45240 | 4 | 66127 | 5690 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction install | 241 | 76174 | 71031 | 46888 | 9 | 104174 | 19063 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction last | 1 | 31506 | 31506 | 28620 | 5 | 51506 | 5150 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction layout | 1 | 33899 | 33899 | 28520 | 5 | 53899 | 7262 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction pass1 | 8662 | 77002 | 70156 | 82880 | 9 | 103410 | 19633 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction snapshot | 1 | 49958 | 49958 | 47260 | 4 | 65958 | 5740 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | compaction write | 4437 | 76223 | 67851 | 71204 | 8 | 103003 | 13997 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | drain | 747 | 50846 | 29433 | 52900 | 7 | 73365 | 11622 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release begin | 6 | 42058 | 36756 | 84560 | 8 | 66058 | 26101 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release install | 9 | 62209 | 57503 | 45168 | 6 | 85295 | 9560 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release last | 1 | 32374 | 32374 | 25420 | 5 | 52374 | 6541 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release layout | 1 | 25721 | 25721 | 21720 | 4 | 41721 | 7201 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release pass1 | 5477 | 76435 | 70473 | 82880 | 7 | 98128 | 19871 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | release write | 2343 | 74724 | 67247 | 63504 | 7 | 97365 | 14403 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal flip | 1 | 9293 | 9293 | 8520 | 5 | 29293 | 900 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal install | 2150 | 70222 | 64937 | 71752 | 10 | 104437 | 24281 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal last | 1 | 35454 | 35454 | 28620 | 6 | 59454 | 6948 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal layout | 1 | 33625 | 33625 | 28520 | 6 | 57625 | 7968 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal pass1 | 12222 | 74478 | 66132 | 95070 | 10 | 101731 | 21021 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal snapshot | 5 | 60874 | 56018 | 86600 | 4 | 68874 | 9479 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | seal write | 3697 | 70699 | 64356 | 61520 | 7 | 98699 | 14359 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction flip | 1 | 5609 | 5609 | 8200 | 3 | 17609 | 902 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction install | 2 | 60433 | 33695 | 58232 | 2 | 68433 | 4801 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction last | 1 | 34992 | 34992 | 28620 | 5 | 54992 | 6541 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction layout | 1 | 32185 | 32185 | 28520 | 3 | 44185 | 7070 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction pass1 | 2676 | 72646 | 65183 | 88858 | 4 | 80646 | 19669 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction snapshot | 1 | 10992 | 10992 | 14600 | 3 | 22992 | 1987 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | compaction write | 3559 | 70649 | 65370 | 55976 | 5 | 84866 | 10567 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | drain | 237 | 49370 | 44803 | 52900 | 7 | 67251 | 10310 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release begin | 6 | 35144 | 31343 | 77000 | 8 | 63265 | 26785 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release install | 1 | 53331 | 53331 | 48064 | 2 | 61331 | 4801 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release last | 1 | 32283 | 32283 | 25420 | 5 | 52283 | 6541 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release layout | 1 | 25363 | 25363 | 21720 | 3 | 37363 | 6983 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release pass1 | 2414 | 73572 | 65281 | 76060 | 4 | 81572 | 19635 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | release write | 2901 | 71817 | 64982 | 46208 | 4 | 83024 | 11256 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal flip | 1 | 9272 | 9272 | 8520 | 5 | 29272 | 900 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal install | 2 | 66894 | 63935 | 104936 | 5 | 86894 | 4868 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal last | 1 | 33738 | 33738 | 28620 | 6 | 57738 | 6948 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal layout | 1 | 32800 | 32800 | 28520 | 4 | 48800 | 7263 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal pass1 | 1480 | 76318 | 65183 | 118740 | 6 | 93586 | 17536 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal snapshot | 8 | 63866 | 56685 | 92450 | 4 | 79866 | 10479 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | seal write | 2609 | 79038 | 64543 | 81000 | 5 | 99038 | 6393 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction flip | 1 | 5625 | 5625 | 8200 | 3 | 17625 | 902 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction install | 7 | 62380 | 60555 | 45168 | 6 | 85199 | 9560 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction last | 1 | 35174 | 35174 | 28620 | 5 | 55174 | 6541 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction layout | 1 | 32670 | 32670 | 28520 | 4 | 48670 | 7207 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction pass1 | 2632 | 80653 | 65601 | 84560 | 7 | 104653 | 20904 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction snapshot | 1 | 10992 | 10992 | 14600 | 3 | 22992 | 1987 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | compaction write | 3608 | 70949 | 65603 | 57312 | 5 | 90945 | 10573 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | drain | 276 | 50129 | 42516 | 52900 | 7 | 70129 | 11292 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release begin | 6 | 48663 | 40249 | 84560 | 8 | 66058 | 26792 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release flip | 1 | 5735 | 5735 | 8200 | 3 | 17735 | 897 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release install | 7 | 62499 | 59500 | 45168 | 6 | 85318 | 9560 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release last | 1 | 32427 | 32427 | 25420 | 5 | 52427 | 6541 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release layout | 1 | 25711 | 25711 | 21720 | 4 | 41711 | 7167 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release pass1 | 2360 | 76436 | 65979 | 76060 | 8 | 96436 | 20904 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | release write | 2937 | 72316 | 64758 | 55816 | 6 | 88177 | 11581 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal begin | 1 | 13864 | 13864 | 11912 | 7 | 41864 | 865 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal flip | 1 | 9288 | 9288 | 8520 | 5 | 29288 | 900 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal install | 8 | 62356 | 60444 | 75408 | 7 | 86356 | 9322 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal last | 1 | 33738 | 33738 | 28620 | 6 | 57738 | 6948 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal layout | 1 | 33543 | 33543 | 28920 | 5 | 53543 | 7979 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal pass1 | 1409 | 81700 | 64519 | 118740 | 9 | 105700 | 20484 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal snapshot | 8 | 65316 | 55457 | 89050 | 4 | 81316 | 10679 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | seal write | 2525 | 78414 | 65757 | 81250 | 6 | 98414 | 10664 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction flip | 1 | 5625 | 5625 | 8200 | 3 | 17625 | 902 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction install | 4 | 62113 | 58442 | 45168 | 6 | 85368 | 9560 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction last | 1 | 35174 | 35174 | 28620 | 5 | 55174 | 6541 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction layout | 1 | 32670 | 32670 | 28520 | 4 | 48670 | 7207 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction pass1 | 2700 | 75628 | 65460 | 88912 | 8 | 98639 | 20904 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction snapshot | 1 | 10992 | 10992 | 14600 | 3 | 22992 | 1987 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | compaction write | 3465 | 71155 | 65701 | 55976 | 5 | 89810 | 10573 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | drain | 265 | 52200 | 43827 | 52900 | 7 | 72200 | 11292 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release begin | 6 | 47939 | 41331 | 87760 | 8 | 66058 | 26792 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release flip | 1 | 5735 | 5735 | 8200 | 3 | 17735 | 897 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release install | 4 | 62232 | 56507 | 45168 | 6 | 85487 | 9560 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release last | 1 | 32427 | 32427 | 25420 | 5 | 52427 | 6541 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release layout | 1 | 25711 | 25711 | 21720 | 4 | 41711 | 7167 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release pass1 | 2459 | 74413 | 64743 | 76060 | 8 | 97836 | 20904 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | release write | 2824 | 72308 | 65327 | 55816 | 5 | 85419 | 11262 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal flip | 1 | 9288 | 9288 | 8520 | 5 | 29288 | 900 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal install | 5 | 62085 | 59310 | 75408 | 6 | 85672 | 9322 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal last | 1 | 33738 | 33738 | 28620 | 6 | 57738 | 6948 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal layout | 1 | 32800 | 32800 | 28520 | 4 | 48800 | 7263 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal pass1 | 1639 | 74470 | 65461 | 118740 | 9 | 98715 | 20484 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal snapshot | 8 | 62834 | 57413 | 91750 | 4 | 70834 | 10509 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | seal write | 2615 | 78065 | 64798 | 81162 | 6 | 98065 | 10931 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | drain | 236 | 46674 | 33248 | 42440 | 8 | 71681 | 7522 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal begin | 2 | 18779 | 16131 | 11912 | 7 | 46779 | 865 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal flip | 2 | 6742 | 6742 | 8200 | 5 | 26742 | 900 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal install | 168 | 65548 | 62127 | 63992 | 9 | 100227 | 9423 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal last | 14 | 63499 | 59746 | 67320 | 5 | 70099 | 6948 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal layout | 29 | 66577 | 61969 | 66920 | 10 | 102837 | 7990 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal pass1 | 18032 | 75117 | 63294 | 105070 | 9 | 103728 | 20638 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal snapshot | 2 | 54799 | 54799 | 67900 | 4 | 70799 | 10179 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | seal write | 49200 | 76480 | 63939 | 82204 | 6 | 96516 | 11903 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction flip | 1 | 5657 | 5657 | 8200 | 3 | 17657 | 902 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction install | 10 | 63381 | 61021 | 45168 | 6 | 87381 | 9560 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction last | 1 | 35184 | 35184 | 28620 | 5 | 55184 | 6541 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction layout | 1 | 34534 | 34534 | 32520 | 5 | 54534 | 8028 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction pass1 | 3919 | 76000 | 66634 | 84560 | 8 | 98970 | 20146 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction snapshot | 1 | 10992 | 10992 | 14600 | 3 | 22992 | 1987 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | compaction write | 6458 | 71947 | 65689 | 61880 | 6 | 89173 | 10831 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | drain | 280 | 49370 | 42109 | 52900 | 7 | 69288 | 11292 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release begin | 6 | 42058 | 36871 | 84560 | 8 | 66058 | 26802 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release flip | 1 | 5767 | 5767 | 8200 | 3 | 17767 | 897 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release install | 10 | 63502 | 60321 | 45168 | 6 | 87502 | 9560 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release last | 1 | 32439 | 32439 | 25420 | 5 | 52439 | 6541 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release layout | 1 | 27576 | 27576 | 25720 | 4 | 43576 | 8084 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release pass1 | 3768 | 73957 | 66991 | 76060 | 7 | 93472 | 20146 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | release write | 5965 | 71850 | 66043 | 61880 | 6 | 88921 | 11556 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal flip | 1 | 9318 | 9318 | 8520 | 5 | 29318 | 900 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal install | 11 | 62802 | 60569 | 69008 | 7 | 88972 | 9322 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal last | 1 | 33736 | 33736 | 28620 | 6 | 57736 | 6948 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal layout | 1 | 34608 | 34608 | 32520 | 5 | 54608 | 7911 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal pass1 | 2491 | 74666 | 64368 | 118740 | 9 | 98954 | 21196 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal snapshot | 8 | 61684 | 58410 | 90650 | 4 | 69684 | 10429 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | seal write | 4996 | 79472 | 64851 | 81362 | 6 | 99472 | 10664 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction install | 2 | 62815 | 49494 | 45168 | 4 | 78815 | 5337 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction last | 1 | 34353 | 34353 | 27240 | 6 | 58353 | 6541 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction layout | 1 | 25376 | 25376 | 21640 | 3 | 37376 | 7135 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction pass1 | 1553 | 71833 | 65461 | 177080 | 7 | 98781 | 15440 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | compaction write | 543 | 71290 | 65592 | 68916 | 5 | 85331 | 10664 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | drain | 33 | 47426 | 26622 | 42840 | 7 | 63426 | 7817 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release begin | 6 | 47939 | 41524 | 87760 | 8 | 66058 | 26067 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release install | 2 | 60822 | 45529 | 44000 | 4 | 76822 | 5337 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release last | 1 | 31611 | 31611 | 24040 | 6 | 55611 | 6541 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release layout | 1 | 19433 | 19433 | 18440 | 3 | 31433 | 7135 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release pass1 | 1410 | 74197 | 63942 | 165080 | 8 | 98469 | 15440 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | release write | 545 | 71898 | 68939 | 59410 | 5 | 87553 | 11327 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal install | 3 | 62352 | 49274 | 67304 | 4 | 77615 | 4931 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal pass1 | 1406 | 76931 | 64388 | 181080 | 7 | 104931 | 16870 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | seal write | 799 | 71102 | 64806 | 52716 | 5 | 85742 | 10931 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction flip | 1 | 5587 | 5587 | 8200 | 3 | 17587 | 902 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction install | 3 | 62548 | 43978 | 45168 | 6 | 84797 | 9560 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction last | 1 | 34353 | 34353 | 27240 | 6 | 58353 | 6541 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction layout | 1 | 25376 | 25376 | 21640 | 3 | 37376 | 7135 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction pass1 | 1578 | 73853 | 65435 | 161080 | 8 | 98763 | 17080 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction snapshot | 1 | 10988 | 10988 | 14600 | 3 | 22988 | 1987 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | compaction write | 545 | 71284 | 66372 | 71316 | 5 | 85331 | 10906 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | drain | 46 | 46634 | 30103 | 42440 | 7 | 58634 | 7773 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release begin | 6 | 47939 | 41645 | 88160 | 8 | 66058 | 26067 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release install | 2 | 62674 | 58798 | 45168 | 6 | 78923 | 9560 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release last | 1 | 31611 | 31611 | 24040 | 6 | 55611 | 6541 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release layout | 1 | 19433 | 19433 | 18440 | 3 | 31433 | 7135 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release pass1 | 1442 | 73295 | 63683 | 169080 | 8 | 98889 | 17080 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | release write | 547 | 73857 | 69706 | 61410 | 5 | 87286 | 11595 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal install | 3 | 61882 | 59252 | 85512 | 6 | 85632 | 9322 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal pass1 | 1455 | 72705 | 64473 | 173080 | 10 | 103599 | 16870 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | seal write | 814 | 71530 | 65005 | 55116 | 5 | 86009 | 10664 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction begin | 2 | 19599 | 17303 | 17712 | 7 | 47599 | 13479 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction flip | 2 | 11135 | 8361 | 13600 | 6 | 35135 | 4734 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction install | 10 | 64814 | 59251 | 45168 | 6 | 87050 | 9560 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction last | 2 | 31785 | 29781 | 24040 | 6 | 55785 | 6541 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction layout | 2 | 24668 | 21987 | 23840 | 6 | 48668 | 7189 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction pass1 | 4373 | 75681 | 66753 | 177080 | 10 | 103520 | 17108 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction snapshot | 2 | 16295 | 13641 | 20000 | 6 | 40295 | 5050 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | compaction write | 1395 | 74980 | 69625 | 81316 | 5 | 88296 | 12631 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | drain | 90 | 53098 | 30934 | 62480 | 7 | 65271 | 9894 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release begin | 6 | 42058 | 37372 | 84560 | 8 | 66058 | 26067 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release flip | 1 | 5713 | 5713 | 8200 | 3 | 17713 | 897 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release install | 3209 | 73222 | 66280 | 60320 | 8 | 94792 | 17595 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release last | 1 | 33403 | 33403 | 14840 | 7 | 61403 | 12483 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release layout | 1 | 19487 | 19487 | 18440 | 3 | 31487 | 7189 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release pass1 | 2009 | 74938 | 65798 | 177080 | 7 | 102938 | 17108 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release snapshot | 1 | 11114 | 11114 | 14600 | 3 | 23114 | 2118 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | release write | 822 | 69266 | 65852 | 59410 | 5 | 82879 | 10498 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal flip | 1 | 8900 | 8900 | 8360 | 5 | 28900 | 900 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal install | 3 | 62059 | 55774 | 69008 | 6 | 86059 | 9322 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal last | 1 | 31847 | 31847 | 25340 | 6 | 55847 | 6948 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal layout | 1 | 25448 | 25448 | 21640 | 3 | 37448 | 7189 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal pass1 | 1526 | 72705 | 64900 | 181080 | 10 | 104165 | 16870 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal snapshot | 6 | 64454 | 57723 | 83600 | 4 | 72454 | 9729 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | seal write | 819 | 72051 | 64170 | 52716 | 5 | 86009 | 10664 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction begin | 1 | 21798 | 21798 | 12312 | 7 | 49798 | 19653 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction flip | 1 | 49928 | 49928 | 45240 | 4 | 65928 | 5690 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction install | 74 | 66519 | 63443 | 70248 | 8 | 96754 | 13621 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction last | 182 | 70807 | 65395 | 64040 | 5 | 78693 | 13397 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction layout | 36 | 68915 | 64450 | 67320 | 7 | 92915 | 13408 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction pass1 | 13332 | 76936 | 65594 | 106267 | 8 | 99477 | 21270 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction snapshot | 1 | 27376 | 27376 | 23280 | 8 | 59376 | 8891 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | compaction write | 27718 | 74727 | 65599 | 111080 | 6 | 97560 | 17077 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | drain | 713 | 60334 | 43621 | 58720 | 7 | 75688 | 16412 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release begin | 38 | 42486 | 35327 | 84560 | 8 | 66486 | 25397 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release install | 50 | 65488 | 61244 | 67480 | 6 | 89488 | 13621 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release last | 10 | 66861 | 57284 | 64040 | 5 | 78861 | 13397 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release layout | 20 | 67045 | 59150 | 66920 | 7 | 89858 | 13408 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release pass1 | 10708 | 74744 | 65046 | 82480 | 7 | 99093 | 19593 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release snapshot | 1 | 19120 | 19120 | 14600 | 5 | 39120 | 3474 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | release write | 15558 | 75383 | 64548 | 62440 | 6 | 99383 | 16214 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal begin | 1 | 18006 | 18006 | 16612 | 8 | 50006 | 11949 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal flip | 1 | 28929 | 28929 | 15080 | 5 | 48929 | 26851 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal install | 115 | 79777 | 62152 | 62072 | 11 | 107777 | 19264 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal last | 40 | 72455 | 65010 | 114370 | 5 | 84409 | 13397 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal layout | 37 | 68630 | 63057 | 66920 | 9 | 99759 | 13408 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal pass1 | 11957 | 75847 | 64838 | 109034 | 9 | 110158 | 26114 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal snapshot | 1 | 41697 | 41697 | 39550 | 6 | 65697 | 10579 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | seal write | 21634 | 82383 | 65179 | 97592 | 6 | 106383 | 13384 |
| Nat and Text values whose holders' compares run over their whole length | compaction begin | 1 | 15008 | 15008 | 12312 | 5 | 35008 | 865 |
| Nat and Text values whose holders' compares run over their whole length | compaction flip | 1 | 5546 | 5546 | 8200 | 3 | 17546 | 902 |
| Nat and Text values whose holders' compares run over their whole length | compaction install | 2 | 62350 | 55193 | 45168 | 4 | 78350 | 4938 |
| Nat and Text values whose holders' compares run over their whole length | compaction last | 1 | 35144 | 35144 | 28620 | 5 | 55144 | 6541 |
| Nat and Text values whose holders' compares run over their whole length | compaction layout | 1 | 32724 | 32724 | 28520 | 4 | 48724 | 7228 |
| Nat and Text values whose holders' compares run over their whole length | compaction pass1 | 3930 | 78326 | 67287 | 104710 | 7 | 90480 | 17104 |
| Nat and Text values whose holders' compares run over their whole length | compaction snapshot | 1 | 10992 | 10992 | 14600 | 3 | 22992 | 1987 |
| Nat and Text values whose holders' compares run over their whole length | compaction write | 5301 | 73852 | 65744 | 59604 | 7 | 100664 | 14403 |
| Nat and Text values whose holders' compares run over their whole length | drain | 287 | 53141 | 44589 | 54800 | 7 | 69736 | 13210 |
| Nat and Text values whose holders' compares run over their whole length | release begin | 6 | 47939 | 42341 | 87760 | 8 | 63297 | 26817 |
| Nat and Text values whose holders' compares run over their whole length | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| Nat and Text values whose holders' compares run over their whole length | release install | 2 | 62476 | 51262 | 45168 | 4 | 78476 | 4936 |
| Nat and Text values whose holders' compares run over their whole length | release last | 1 | 32402 | 32402 | 25420 | 5 | 52402 | 6541 |
| Nat and Text values whose holders' compares run over their whole length | release layout | 1 | 26720 | 26720 | 25320 | 4 | 42720 | 7228 |
| Nat and Text values whose holders' compares run over their whole length | release pass1 | 3759 | 74911 | 67478 | 100710 | 6 | 95049 | 17104 |
| Nat and Text values whose holders' compares run over their whole length | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| Nat and Text values whose holders' compares run over their whole length | release write | 5471 | 78499 | 66459 | 49648 | 7 | 106499 | 11101 |
| Nat and Text values whose holders' compares run over their whole length | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| Nat and Text values whose holders' compares run over their whole length | seal flip | 1 | 9193 | 9193 | 8520 | 5 | 29193 | 900 |
| Nat and Text values whose holders' compares run over their whole length | seal install | 3 | 62070 | 58524 | 81808 | 6 | 77875 | 4929 |
| Nat and Text values whose holders' compares run over their whole length | seal last | 1 | 33738 | 33738 | 28620 | 6 | 57738 | 6948 |
| Nat and Text values whose holders' compares run over their whole length | seal layout | 1 | 32854 | 32854 | 28520 | 4 | 48854 | 7284 |
| Nat and Text values whose holders' compares run over their whole length | seal pass1 | 3417 | 74397 | 68269 | 119140 | 7 | 93975 | 20004 |
| Nat and Text values whose holders' compares run over their whole length | seal snapshot | 7 | 61219 | 57887 | 85300 | 4 | 69219 | 9579 |
| Nat and Text values whose holders' compares run over their whole length | seal write | 2882 | 75839 | 64619 | 85022 | 8 | 97788 | 15505 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction begin | 3 | 22675 | 19919 | 19872 | 8 | 54675 | 14065 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction flip | 3 | 50127 | 24295 | 45240 | 6 | 66127 | 5690 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction install | 2622 | 76112 | 72340 | 45720 | 9 | 99805 | 19639 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction last | 1482 | 65042 | 63513 | 65280 | 6 | 78636 | 13018 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction layout | 3 | 33943 | 33905 | 28520 | 5 | 53943 | 7284 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction pass1 | 37401 | 76224 | 59543 | 84960 | 9 | 103740 | 22456 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction snapshot | 3 | 19072 | 16997 | 22880 | 7 | 47072 | 6478 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | compaction write | 22616 | 75029 | 66547 | 66304 | 8 | 101688 | 16002 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | drain | 1455 | 51695 | 21053 | 54800 | 7 | 74019 | 11378 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release begin | 6 | 42058 | 37190 | 84560 | 8 | 66058 | 26101 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release flip | 1 | 5672 | 5672 | 8200 | 3 | 17672 | 897 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release install | 95 | 63465 | 61206 | 45168 | 6 | 87465 | 9560 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release last | 196 | 64337 | 64197 | 50560 | 4 | 79002 | 6541 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release layout | 1 | 25687 | 25687 | 21720 | 4 | 41687 | 7167 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release pass1 | 5727 | 73800 | 55765 | 76060 | 8 | 97216 | 20158 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release snapshot | 1 | 11118 | 11118 | 14600 | 3 | 23118 | 2118 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | release write | 2947 | 72151 | 66095 | 55816 | 7 | 92322 | 15955 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal begin | 1 | 12774 | 12774 | 11912 | 5 | 32774 | 865 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal flip | 1 | 9293 | 9293 | 8520 | 5 | 29293 | 900 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal install | 14692 | 74961 | 53454 | 64928 | 8 | 94693 | 23857 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal last | 484 | 63935 | 61931 | 62480 | 6 | 87935 | 11488 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal layout | 1 | 32909 | 32909 | 28520 | 5 | 52909 | 7284 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal pass1 | 11193 | 74772 | 55213 | 118540 | 9 | 99585 | 19829 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal snapshot | 7 | 63729 | 60860 | 91050 | 4 | 74126 | 9779 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | seal write | 5110 | 70030 | 64298 | 55816 | 7 | 96807 | 15735 |

Most instructions of any unit: 82383. Most changed pages: 11. Most charged: 114766.
Longest stretch between two meter readings of a unit: 26851.

The meter's decisions, checked in a module built with the crate's `meter-notes`: 21480683 decisions, the work each let begin within the look-ahead and the pages it took.

Each scenario's first group: its seal and the drain after it, per row of the group (units cut by their own estimates only; charged: instructions and 4000 per changed page):

| scenario | rows | seal units | seal instructions per row | seal charged per row | drain units | drain instructions per row | drain charged per row | seal and drain instructions per row | seal and drain charged per row |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| mixed rows: seal, compaction of deletes and patches, a drop | 4096 | 2681 | 17999 | 21749 | 549 | 2682 | 3424 | 20682 | 25174 |
| 64 KiB-class values: seal, compaction of patches | 64 | 1234 | 811565 | 1020503 | 705 | 252643 | 316893 | 1064208 | 1337396 |
| 621 leaves: seal, compaction of 600 patch units | 480 | 10260 | 533297 | 654955 | 69 | 1652 | 5094 | 534950 | 660050 |
| values sharing long prefixes: seal, compaction | 40 | 11596 | 9084397 | 10503297 | 355 | 204245 | 256245 | 9288642 | 10759542 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | 503 | 31701 | 1527807 | 1902781 | 196 | 8364 | 14742 | 1536172 | 1917523 |
| rows of three schema versions: seal, column release, patches | 350 | 84700 | 8753311 | 10184077 | 376 | 22082 | 32220 | 8775394 | 10216297 |
| writes between units, clear in a compaction and in a seal | 4096 | 11343 | 94380 | 123870 | 779 | 3783 | 5433 | 98164 | 129304 |
| 170 running records: seal, compaction of patches and deletes, column release | 454 | 9922 | 609937 | 750501 | 337 | 19822 | 27293 | 629760 | 777795 |
| 120 Bool and tag leaves: seal, a group dropped | 1200 | 6350 | 148344 | 181027 | 61 | 601 | 1808 | 148945 | 182835 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | 1024 | 134930 | 3718723 | 4253434 | 259 | 5786 | 6907 | 3724509 | 4260341 |
| the reference row: seal | 4096 | 1513 | 9436 | 11548 | 39 | 90 | 353 | 9526 | 11901 |
| every flat leaf kind: seal, compaction, column release, a drop | 256 | 4890 | 640620 | 751088 | 96 | 5960 | 12851 | 646581 | 763940 |
| every flat leaf kind, columns added and rows written in the seal's install | 256 | 7960 | 1089531 | 1343046 | 172 | 12118 | 22368 | 1101649 | 1365415 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | 4096 | 4886 | 34989 | 41569 | 571 | 2795 | 3567 | 37785 | 45136 |
| mixed rows, the table opened with a newer schema in its seal | 4096 | 2622 | 17775 | 21592 | 551 | 2687 | 3440 | 20463 | 25033 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | 4096 | 10284 | 99826 | 119925 | 30 | 105 | 247 | 99931 | 120173 |
| Nat and Int sums past 65,535 bytes: seal, compaction | 4096 | 100376 | 774490 | 958346 | 594 | 2854 | 3748 | 777344 | 962095 |
| 120 Bool and tag leaves: compaction, column release | 1200 | 6350 | 148321 | 181005 | 61 | 601 | 1808 | 148923 | 182813 |
| 170 Text running records, Bool and tag leaves: compaction, column release | 386 | 17035 | 1309857 | 1578427 | 955 | 76406 | 92116 | 1386264 | 1670544 |
| 90 Text running records, Bool and tag leaves: compaction, column release | 661 | 15689 | 733920 | 875536 | 693 | 26886 | 36593 | 760807 | 912129 |
| 60 Text running records, Bool and tag leaves: compaction, column release | 885 | 13412 | 462985 | 553001 | 522 | 13541 | 20082 | 476527 | 573083 |
| a 200-case tag: a drop | 4096 | 2773 | 23589 | 28510 | 23 | 53 | 206 | 23642 | 28716 |
| a 200-case tag, every row updated: a drop | 4096 | 2773 | 23589 | 28510 | 23 | 53 | 206 | 23642 | 28716 |
| a 200-case tag, a third of the rows updated: a drop | 4096 | 2773 | 23589 | 28510 | 23 | 53 | 206 | 23642 | 28716 |
| a 200-case tag: compaction, column release | 4096 | 2773 | 23589 | 28510 | 23 | 53 | 206 | 23642 | 28716 |
| a 40-case tag: compaction, column release | 4096 | 2423 | 20952 | 24445 | 14 | 32 | 126 | 20984 | 24572 |
| a 17-case tag: compaction, column release | 4096 | 556 | 3613 | 4372 | 13 | 30 | 118 | 3643 | 4491 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | 1715 | 43822 | 1162088 | 1366193 | 847 | 9062 | 17167 | 1171151 | 1383361 |
| narrow rows, a Nat key and two Nats: seal | 4096 | 670 | 3846 | 4698 | 20 | 46 | 181 | 3892 | 4880 |
| a 17-case tag, 64 deletes after a compaction's snapshot | 4096 | 486 | 3145 | 3759 | 13 | 30 | 118 | 3175 | 3878 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | 4096 | 486 | 3145 | 3759 | 13 | 30 | 118 | 3175 | 3878 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | 4096 | 486 | 3038 | 3652 | 13 | 30 | 118 | 3068 | 3771 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | 4096 | 2354 | 23413 | 26767 | 15 | 34 | 134 | 23447 | 26902 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | 4096 | 2695 | 22835 | 26925 | 14 | 32 | 122 | 22867 | 27048 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | 4096 | 4077 | 40341 | 48119 | 26 | 95 | 209 | 40437 | 48329 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | 4096 | 4077 | 40341 | 48119 | 26 | 95 | 209 | 40437 | 48329 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | 4096 | 4077 | 40341 | 48119 | 26 | 95 | 209 | 40437 | 48329 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | 4096 | 4077 | 40341 | 48119 | 26 | 95 | 209 | 40437 | 48329 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | 4096 | 4077 | 40341 | 48119 | 26 | 95 | 209 | 40437 | 48329 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | 4096 | 6094 | 71868 | 91207 | 91 | 387 | 649 | 72255 | 91856 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | 4096 | 4598 | 47925 | 58655 | 26 | 95 | 209 | 48020 | 58865 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | 4096 | 187132 | 1419247 | 1766019 | 748 | 3415 | 4940 | 1422662 | 1770960 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | 4096 | 173260 | 1308163 | 1645451 | 805 | 3476 | 5432 | 1311640 | 1650884 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | 2625 | 30271 | 513073 | 599378 | 794 | 6633 | 8376 | 519706 | 607755 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | 4096 | 6927 | 71373 | 81741 | 549 | 2707 | 3402 | 74081 | 85143 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | 4096 | 6534 | 67913 | 78148 | 551 | 2711 | 3419 | 70624 | 81568 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | 4096 | 7348 | 75203 | 86105 | 551 | 2712 | 3419 | 77915 | 89524 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | 426 | 51668 | 5276430 | 6060853 | 192 | 9001 | 16879 | 5285431 | 6077732 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | 4096 | 11913 | 126105 | 145323 | 556 | 2723 | 3465 | 128829 | 148789 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | 4096 | 3883 | 38076 | 46107 | 25 | 94 | 205 | 38171 | 46312 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | 4096 | 3949 | 39073 | 47184 | 26 | 95 | 209 | 39168 | 47394 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | 4096 | 4077 | 40268 | 48046 | 26 | 95 | 209 | 40364 | 48256 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | 179 | 57077 | 13095663 | 15254523 | 648 | 95249 | 118378 | 13190913 | 15372902 |
| Nat and Text values whose holders' compares run over their whole length | 4096 | 11785 | 118880 | 138840 | 553 | 2921 | 4148 | 121801 | 142988 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | 4096 | 48941 | 464452 | 557998 | 1487 | 5737 | 10894 | 470190 | 568893 |

Each scenario's first group: its seal and the drain after it, per row of the group (units cut by the replica's count as their host meter; charged: instructions and 4000 per changed page):

| scenario | rows | seal units | seal instructions per row | seal charged per row | drain units | drain instructions per row | drain charged per row | seal and drain instructions per row | seal and drain charged per row |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| mixed rows: seal, compaction of deletes and patches, a drop | 4096 | 599 | 10200 | 11245 | 241 | 2568 | 3009 | 12769 | 14254 |
| 64 KiB-class values: seal, compaction of patches | 64 | 644 | 641706 | 792893 | 353 | 251146 | 291396 | 892852 | 1084290 |
| 621 leaves: seal, compaction of 600 patch units | 480 | 2620 | 376206 | 419231 | 63 | 1704 | 5070 | 377910 | 424301 |
| values sharing long prefixes: seal, compaction | 40 | 2368 | 3812928 | 4260228 | 180 | 203300 | 236800 | 4016228 | 4497028 |
| 1,024 leaves: seal, compaction of 700 patch units and deletes, a drop | 503 | 6619 | 918588 | 1028322 | 108 | 8502 | 12224 | 927091 | 1040546 |
| rows of three schema versions: seal, column release, patches | 350 | 33947 | 6288168 | 6898088 | 183 | 19622 | 26628 | 6307790 | 6924716 |
| writes between units, clear in a compaction and in a seal | 4096 | 4262 | 70424 | 82529 | 406 | 3731 | 5016 | 74156 | 87546 |
| 170 running records: seal, compaction of patches and deletes, column release | 454 | 2581 | 401213 | 445284 | 195 | 19859 | 24282 | 421073 | 469567 |
| 120 Bool and tag leaves: seal, a group dropped | 1200 | 1586 | 93436 | 103052 | 57 | 639 | 1842 | 94075 | 104895 |
| 64 KiB values sharing a 64 KiB prefix: seal, compaction | 1024 | 20420 | 1242342 | 1326393 | 111 | 5590 | 6145 | 1247933 | 1332538 |
| the reference row: seal | 4096 | 341 | 5857 | 6459 | 39 | 96 | 360 | 5953 | 6819 |
| every flat leaf kind: seal, compaction, column release, a drop | 256 | 1892 | 479229 | 531057 | 71 | 5982 | 12279 | 485212 | 543337 |
| every flat leaf kind, columns added and rows written in the seal's install | 256 | 3160 | 805601 | 916508 | 108 | 12072 | 20822 | 817674 | 937330 |
| mixed rows with 1 MiB Text and Blob: seal, compaction, column release, a drop | 4096 | 1147 | 18642 | 20649 | 250 | 2676 | 3134 | 21318 | 23784 |
| mixed rows, the table opened with a newer schema in its seal | 4096 | 655 | 11218 | 12381 | 243 | 2574 | 3027 | 13792 | 15409 |
| tags of 300 and 5,000 cases: seal, a drop, updates to cases outside the dictionaries | 4096 | 5667 | 89743 | 101511 | 25 | 107 | 240 | 89850 | 101752 |
| Nat and Int sums past 65,535 bytes: seal, compaction | 4096 | 44242 | 596218 | 679434 | 273 | 2739 | 3321 | 598958 | 682755 |
| 120 Bool and tag leaves: compaction, column release | 1200 | 1577 | 93254 | 102747 | 57 | 639 | 1842 | 93893 | 104590 |
| 170 Text running records, Bool and tag leaves: compaction, column release | 386 | 4709 | 847127 | 942288 | 622 | 76337 | 86700 | 923465 | 1028988 |
| 90 Text running records, Bool and tag leaves: compaction, column release | 661 | 4622 | 489994 | 542538 | 406 | 26681 | 33356 | 516675 | 575895 |
| 60 Text running records, Bool and tag leaves: compaction, column release | 885 | 3892 | 307281 | 340221 | 287 | 13398 | 17764 | 320679 | 357986 |
| a 200-case tag: a drop | 4096 | 1240 | 19480 | 21871 | 23 | 56 | 210 | 19537 | 22081 |
| a 200-case tag, every row updated: a drop | 4096 | 1240 | 19480 | 21871 | 23 | 56 | 210 | 19537 | 22081 |
| a 200-case tag, a third of the rows updated: a drop | 4096 | 1240 | 19480 | 21871 | 23 | 56 | 210 | 19537 | 22081 |
| a 200-case tag: compaction, column release | 4096 | 1240 | 19480 | 21871 | 23 | 56 | 210 | 19537 | 22081 |
| a 40-case tag: compaction, column release | 4096 | 1118 | 17822 | 19540 | 14 | 34 | 129 | 17856 | 19669 |
| a 17-case tag: compaction, column release | 4096 | 134 | 2244 | 2463 | 13 | 32 | 120 | 2276 | 2584 |
| Nat and Int values of 60,000 bytes updated in a seal and a compaction | 1715 | 26376 | 1032870 | 1162179 | 570 | 9152 | 15633 | 1042022 | 1177812 |
| narrow rows, a Nat key and two Nats: seal | 4096 | 137 | 2290 | 2498 | 20 | 49 | 184 | 2340 | 2683 |
| a 17-case tag, 64 deletes after a compaction's snapshot | 4096 | 121 | 2009 | 2192 | 13 | 32 | 120 | 2042 | 2313 |
| a 17-case tag, 64 updates among its cases after a compaction's snapshot | 4096 | 121 | 2009 | 2192 | 13 | 32 | 120 | 2042 | 2313 |
| a 17-case tag on 8 cases, 9 updates to the others after a compaction's snapshot | 4096 | 115 | 1915 | 2087 | 13 | 32 | 120 | 1947 | 2208 |
| a 64-case tag on 32 cases, 32 updates to the others after a compaction's snapshot | 4096 | 1341 | 21054 | 23198 | 15 | 36 | 137 | 21090 | 23335 |
| a 300-case tag on 150 cases, 150 updates to the others after a compaction's snapshot | 4096 | 1183 | 18812 | 20695 | 14 | 34 | 125 | 18847 | 20820 |
| a 5,000-case tag on 1,000 cases, 8 updates to the others after a compaction's snapshot | 4096 | 2362 | 37241 | 41893 | 21 | 98 | 207 | 37339 | 42100 |
| a 5,000-case tag on 1,000 cases, 256 updates to the others after a compaction's snapshot | 4096 | 2362 | 37241 | 41893 | 21 | 98 | 207 | 37339 | 42100 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a compaction's snapshot | 4096 | 2362 | 37241 | 41893 | 21 | 98 | 207 | 37339 | 42100 |
| a 5,000-case tag on 1,000 cases, 3,500 updates to the others after a compaction's snapshot | 4096 | 2362 | 37241 | 41893 | 21 | 98 | 207 | 37339 | 42100 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a column release's snapshot | 4096 | 2362 | 37241 | 41893 | 21 | 98 | 207 | 37339 | 42100 |
| a 5,000-case tag on 1,000 cases, 2,000 updates to the others after a seal's snapshot | 4096 | 4254 | 68595 | 84132 | 55 | 382 | 608 | 68977 | 84740 |
| a 5,000-case tag on 1,000 cases, 512 updates to the others after a seal's snapshot | 4096 | 2758 | 43880 | 50810 | 21 | 98 | 207 | 43978 | 51018 |
| 100,000-byte Nat and Int sums that carry across their length, updated after the snapshots | 4096 | 86764 | 1094760 | 1260681 | 399 | 3320 | 4516 | 1098080 | 1265198 |
| 150,000-byte Nat and Int values whose low groups are zero, updated after the snapshots | 4096 | 80720 | 1033642 | 1193329 | 464 | 3390 | 5014 | 1037033 | 1198343 |
| 50,000-byte Nat and Int values that differ in their lowest group, updated after the snapshots | 2625 | 18078 | 451878 | 506620 | 383 | 6505 | 7618 | 458383 | 514239 |
| an Int whose least value is a 3,000-byte negative Int, its lowest group that is not zero 2,040 bytes in | 4096 | 4103 | 64828 | 71209 | 236 | 2589 | 2974 | 67417 | 74184 |
| an Int whose least value is a 40,000-byte negative Int, its lowest group that is not zero 1,000 bytes in | 4096 | 3954 | 62979 | 69362 | 238 | 2593 | 2992 | 65573 | 72354 |
| an Int whose least value is a 20,000-byte negative Int, its lowest group that is not zero 30 bytes in | 4096 | 4271 | 67769 | 74413 | 238 | 2594 | 2992 | 70363 | 77406 |
| 120 Int leaves, each with one 6,000-byte negative value: two groups sealed | 426 | 33617 | 5026793 | 5551816 | 118 | 9210 | 14421 | 5036003 | 5566238 |
| Nat and Int values of 39,600 and 19,800 bytes on two rows: seal, compaction, column release | 4096 | 7510 | 118545 | 130822 | 243 | 2606 | 3039 | 121152 | 133862 |
| a 5,000-case tag on its 840 highest cases: seal, compaction, column release | 4096 | 2218 | 34883 | 39755 | 20 | 96 | 203 | 34980 | 39959 |
| a 5,000-case tag on its 1,180 highest cases: seal, compaction, column release | 4096 | 2282 | 35968 | 40924 | 21 | 98 | 207 | 36066 | 41131 |
| a 5,000-case tag on 1,000 cases, two waves of updates to other cases during a column release | 4096 | 2358 | 37154 | 41799 | 21 | 98 | 207 | 37252 | 42006 |
| 300 optional Text columns, half dropped and as many Nat added before the first row | 179 | 33786 | 12276737 | 13617765 | 341 | 92608 | 104764 | 12369346 | 13722530 |
| Nat and Text values whose holders' compares run over their whole length | 4096 | 6313 | 102582 | 114091 | 252 | 2836 | 3700 | 105418 | 117791 |
| Nat and Int values of 18 lengths at the edges of their pieces, 167 to 524,288 bytes | 4096 | 31489 | 430271 | 491993 | 1055 | 5736 | 10462 | 436007 | 502456 |
