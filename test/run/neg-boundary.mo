//MOC-FLAG -A=M0194
let _ : Int8 = -128;
let _ : Int16 = -32768;
let _ : Int32 = -2_147_483_648;
let _ : Int64 = -9_223_372_036_854_775_808;
do {
    assert (switch (-128 : Int8) { case (-128) { true }; case _ { false }});
    func foo(i : Int8) = assert (switch i { case (-128) { true }; case _ { false }});
    func bar(i : Int8) = assert (switch i { case (-128 : Int8) { true }; case _ { false }});
};
do {
    assert (switch (-32768 : Int16) { case (-32768) { true }; case _ { false }});
    func foo(i : Int16) = assert (switch i { case (-32768) { true }; case _ { false }});
    func bar(i : Int16) = assert (switch i { case (-32768 : Int16) { true }; case _ { false }});
};
do {
    assert (switch (-2_147_483_648 : Int32) { case (-2_147_483_648) { true }; case _ { false }});
    func foo(i : Int32) = assert (switch i { case (-2_147_483_648) { true }; case _ { false }});
    func bar(i : Int32) = assert (switch i { case (-2_147_483_648 : Int32) { true }; case _ { false }});
};
do {
    assert (switch (-9_223_372_036_854_775_808 : Int64) { case (-9_223_372_036_854_775_808) { true }; case _ { false }});
    func foo(i : Int64) = assert (switch i { case (-9_223_372_036_854_775_808) { true }; case _ { false }});
    func bar(i : Int64) = assert (switch i { case (-9_223_372_036_854_775_808 : Int64) { true }; case _ { false }});
};
