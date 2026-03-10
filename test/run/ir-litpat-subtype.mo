//MOC-FLAG -A=M0194
func bar (a : Nat) = switch a {
   case (25 : Int) ();   // OK: pattern of supertype accepted
   case _ ();
}

