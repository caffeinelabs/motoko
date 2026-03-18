//MOC-FLAG -A=M0194
func f() {};

let u = ();
ignore (); // redundant
ignore f(); // redundant
ignore u; // redundant
ignore { (); }; // redundant, but failed to detect (needs lower bounds passed down analysis)
ignore (if true () else ()); // redundant, but failed to detect (needs lower bounds passed down analysis)

func g () {
  ignore (); // redundant
  ignore f(); // redundant
  ignore u; // redundant
  ignore { (); }; // redundant, but failed to detect (needs lower bounds passed down analysis)
  ignore (if true () else ()); // redundant, but failed to detect (needs lower bounds passed down analysis)
};
