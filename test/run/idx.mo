//MOC-FLAG -A=M0194
class CO4() {
    public type b = Nat;
    public let a : b = 25;
    public let foo : b = 8;
    public let field : b = 42;
    public let other : b = 83
};

type O1 = { field : Int }; // "\ba\94\93\00"
type O2 = { a: Int; field : Int }; // "a\00\00\00\ba\94\93\00"
type O3 = { a: Int; field : Int; other : Int }; // "a\00\00\00\ba\94\93\00\d0fv6"
type O4 = { a: Int; foo : Int; field : Int; other : Int }; // "a\00\00\00\06\c7M\00\ba\94\93\00\d0fv6"

func go1(o : O1) : () = inner o;
func go2(o : O2) : () = inner o;
func go3(o : O3) : () = inner o;
func go4(o : O4) : () { assert o.a == 25; assert o.field == 42; assert o.other == 83; assert o.foo == 8; inner o };

func inner(o : O1) { assert o.field == 42 };

func go() {
    go1({ field = 42 }); // field: 9671866
    go2({ a = 25; field = 42 });
    go3({ a = 25; field = 42; other = 83 });
    go4({ a = 25; foo = 8; field = 42; other = 83 });
    let co4 = CO4();
    let a = co4.a;
    go4(co4);
    absurdities(co4.foo)
};

func absurdities(inp : Int) {
    if (inp == 1) go1(loop {});
    if (inp == 2) go2(loop {});
    if (inp == 3) go3(loop {});
    if (inp == 4) go4(loop {});

    if (inp == 1) (loop {}).field;
    if (inp == 2) (loop {}).a;
    if (inp == 3) (loop {}).other;
    if (inp == 4) (loop {}).foo;
};

go();
