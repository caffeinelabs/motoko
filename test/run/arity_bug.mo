//MOC-FLAG -A=M0194
func f((i:Int,b:Bool)):((Int,Bool)) { return (i,b);};
let g : ((Int,Bool)) -> ((Int,Bool)) = f;
