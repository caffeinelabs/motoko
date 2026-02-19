//MOC-FLAG -A=M0194
func p((),) {};
func p1(()) {};
func p2((())) {};
func p3(((()))) {};

let ps : [((),) -> ()] = [p, p1, p2, p3];
