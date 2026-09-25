func specials(two : { #c0; #c1 }) {
  switch two {
    case #c2 { () } // note #c2 <> #c1
    case #c2 { assert false } // note #c2 <> #c1
  };
};

specials(#c1)
