// `Check_ir` used to raise `Dom.Clash` here: compiling the actor reused the
// constructor stamps handed out while type-checking it, so a fresh `$` binder
// from `Peer` could equal the scope parameter of `f`. The method counts place
// that collision inside the window where it happened.
actor {
  type Pad = actor {
    p0 : () -> async ();
    p1 : () -> async ();
    p2 : () -> async ();
    p3 : () -> async ();
    p4 : () -> async ();
    p5 : () -> async ();
    p6 : () -> async ();
    p7 : () -> async ();
    p8 : () -> async ();
    p9 : () -> async ();
    p10 : () -> async ();
    p11 : () -> async ();
    p12 : () -> async ();
    p13 : () -> async ();
    p14 : () -> async ();
    p15 : () -> async ();
    p16 : () -> async ();
    p17 : () -> async ();
    p18 : () -> async ();
    p19 : () -> async ();
    p20 : () -> async ();
    p21 : () -> async ();
    p22 : () -> async ();
    p23 : () -> async ();
    p24 : () -> async ();
    p25 : () -> async ();
    p26 : () -> async ();
    p27 : () -> async ();
    p28 : () -> async ();
    p29 : () -> async ();
    p30 : () -> async ();
    p31 : () -> async ();
    p32 : () -> async ();
    p33 : () -> async ();
    p34 : () -> async ();
    p35 : () -> async ();
    p36 : () -> async ();
    p37 : () -> async ();
    p38 : () -> async ();
    p39 : () -> async ();
    p40 : () -> async ();
    p41 : () -> async ();
    p42 : () -> async ();
    p43 : () -> async ();
    p44 : () -> async ();
    p45 : () -> async ();
    p46 : () -> async ();
    p47 : () -> async ();
    p48 : () -> async ();
    p49 : () -> async ();
    p50 : () -> async ();
    p51 : () -> async ();
    p52 : () -> async ();
    p53 : () -> async ();
    p54 : () -> async ();
    p55 : () -> async ();
    p56 : () -> async ();
    p57 : () -> async ();
    p58 : () -> async ();
    p59 : () -> async ();
    p60 : () -> async ();
    p61 : () -> async ();
    p62 : () -> async ();
    p63 : () -> async ();
    p64 : () -> async ();
    p65 : () -> async ();
    p66 : () -> async ()
  };
  type Peer = actor {
    m0 : () -> async ();
    m1 : () -> async ();
    m2 : () -> async ();
    m3 : () -> async ();
    m4 : () -> async ();
    m5 : () -> async ();
    m6 : () -> async ();
    m7 : () -> async ();
    m8 : () -> async ();
    m9 : () -> async ();
    m10 : () -> async ();
    m11 : () -> async ();
    m12 : () -> async ();
    m13 : () -> async ();
    m14 : () -> async ();
    m15 : () -> async ();
    m16 : () -> async ();
    m17 : () -> async ();
    m18 : () -> async ();
    m19 : () -> async ();
    m20 : () -> async ();
    m21 : () -> async ();
    m22 : () -> async ();
    m23 : () -> async ();
    m24 : () -> async ();
    m25 : () -> async ();
    m26 : () -> async ();
    m27 : () -> async ();
    m28 : () -> async ();
    m29 : () -> async ();
    m30 : () -> async ();
    m31 : () -> async ();
    m32 : () -> async ();
    m33 : () -> async ();
    m34 : () -> async ();
    m35 : () -> async ();
    m36 : () -> async ();
    m37 : () -> async ();
    m38 : () -> async ();
    m39 : () -> async ()
  };
  func _f(_ : Peer) : async () {}
}
