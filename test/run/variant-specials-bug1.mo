func specials(two : { #c0; #c1 }) {
  switch two {
    case #c0 { () }
    case #c2 { assert false } // note #c2 <> #c1
  };
};

specials(#c1)
