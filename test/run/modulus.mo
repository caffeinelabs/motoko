do { let a = 13; let b = 5; assert (2 == a / b); assert (3 == a % b) };
do { let a = -13; let b = 5; assert (-2 == a / b); assert (-3 == a % b) };
do { let a = -13; let b = -5; assert (2 == a / b); assert (-3 == a % b) };
do { let a = 13; let b = -5; assert (-2 == a / b); assert (3 == a % b) };

do { let a = 5; let b = 5; assert (1 == a / b); assert (0 == a % b) };
do { let a = -5; let b = 5; assert (-1 == a / b); assert (0 == a % b) };
do { let a = -5; let b = -5; assert (1 == a / b); assert (0 == a % b); };
do { let a = 5; let b = -5; assert (-1 == a / b); assert (0 == a % b) };

do { let a = 0; let b = 5; assert (0 == a / b); assert (0 == a % b) };
do { let a = 0; let b = -5; assert (0 == a / b); assert (0 == a % b) };
