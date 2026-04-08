//MOC-FLAG -A=M0194
actor a {
  public func go() : () {
    let opts = [null, ?42, ?-25];
    let nulls = [null, null];

    let incompatible_objs = [{a = 42}, {b = 42}];
    let objs = [{a = 42}, {b = 42; a = 1}, {a = -25}];

    let tups = [(12, -1), (-42, 25)];
    let tup1s = [(-1,), 25];

    let arrs = [[-42], [25]];

    let incompatible_funcs = [ func (a : [Int]) : Nat = a.size()
                             , func (a : ()) : Int = -42
                             ];

    let poly_funcs = [ func<A> (a : [Int]) : Nat = a.size()
                     , func<A> (a : [Nat]) : Int = -42
                     ];

    let poly_funcs2 = [ func<A> (a : [Int]) : Nat = a.size()
                      , func<B> (a : [Nat]) : Int = -42
                      ];

    let poly_funcs3 = [ func<A, B> (as : [A], b : B) : A = as[0]
                      , func<B, A> (bs : [B], a : A) : B = bs[0]
                      ];

    let funcs = [ func (a : [Int]) : Nat = a.size()
                , func (a : [Nat]) : Int = -42
                ];

    let variant_funcs = [ func (a : {#foo; #bar}) { switch a { case (#foo) (); case (#bar) () } }
                        , func (a : {#baz; #bar}) { switch a { case (#baz) (); case (#bar) () } }
                        ];

    // TODO(gabor), mutable arrays
    let mut_arrs = [[var 42], [var 25], [var 77]]; // boring

    // TODO(gabor), mutable fields, see fail/type-inference.mo:13

    let sh : Any = 42;
    let shareds = [sh, 77, [1, 2, 3]];
    let shared2s = [77, [1, 2, 3], sh];

    let shared_funcs = [ func (a : Int) : Int = a
                       , func (a : Nat) : Nat = 42
                       ];


    func c0(c : async (?Int), d : async (?Nat)) : [async (?Int)] { ignore([c, d]); [c, d] };
    let c1s = [async ?4, async ?-42];


    // recursive objects

    // {
    type A = {x : A};
    type B = {x : B};

    func f(v : {x : {x : B}; b : B}, x : A, y : B, z : {x : B; a : A}) : [A] { ignore([v, x, y, z]); [v, x, y, z] };
    // };

    // {
    type A1 = {x : B1};
    type B1 = {x : A1};

    func f1(x : A1, y : B1) : [A1] { ignore([x, y]); [x, y] };
    // };

    type O = ?O;
    type P = ?P;

    type Q = ?R;
    type R = ?S;
    type S = ?Q;

    func g(o : O, p : P, q : Q, r : R) : [O] { ignore([o, p, q, r]); [o, p, q, r] };



    type Foo<A> = ?(Foo<A>);
    ignore (if true (null : Foo<Int>) else (null : Foo<Bool>));


    type U = { #a : U; #b : Int };
    type V = { #a : V; #b : Nat };

    func v0(u : U, v : V, w : { #a : { #a : V; #b : Nat }; #b : Nat }) : [U] { ignore([u, v, w]); [u, v, w] };


    type G = (Nat, ?G);
    type H = (Int, ?H);

    func g0(g : G, h : H) : [H] { ignore([g, h]); [g, h] };


    type K = [K];
    type L = [L];

    func k0(k : K, l : L) : [L] { ignore([k, l]); [k, l] };


    type K1 = [?(Nat, K1)];
    type L1 = [?(Int, L1)];

    func k1(k : K1, l : L1) : [L1] { ignore([k, l]); [k, l] };


    /*
    type M = [var ?M];
    type N = [?N];

    func m0(m : M, n : N) : [M] { ignore([m, n]); [m, n] };
    */

    type E = Int -> E;
    type F = Nat -> F;

    func f0(e : E, f : F) : [F] { ignore([e, f]); [e, f] };

    type E1 = E1 -> E1;
    type F1 = F1 -> F1;

    func f12(e : E1, f : F1) : [F1] { ignore([e, f]); [e, f] };

    type E2 = F2 -> E2;
    type F2 = E2 -> F2;

    func f2(e : E2, f : F2) : [F2] { ignore([e, f]); [e, f] };
  }
};
a.go(); //OR-CALL ingress go "DIDL\x00\x00"
