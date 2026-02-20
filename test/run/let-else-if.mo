//MOC-FLAG -A=M0194
let x = if true 4 else 5 else { assert false; loop () };

func foo() {
    let 2 = if true 3 else 3 else { return };
    assert false;
};

foo();

func bar() {
    if false let _ = 4 else return;
    assert false;
};

bar();
