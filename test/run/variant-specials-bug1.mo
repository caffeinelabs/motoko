//MOC-FLAG -W=M0145
func specials(two : { #c0; #c1 }) {
  switch (two : { #c0; #c1; #c2 }) {
    case (#c0) ();
    case (#c2) assert false; // note #c2 <> #c1
  };
};

specials(#c1)
