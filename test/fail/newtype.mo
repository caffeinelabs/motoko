// newtype error cases

newtype Time = Int;
newtype Duration = Int;

// cannot assign Int where Time is expected
func m1() { let bad1 : Time = 123 };

// cannot assign Time where Int is expected
func m2() { let bad2 : Int = Time(0) };

// cannot assign Time where Duration is expected (different newtypes over same type)
func m3() { let bad3 : Duration = Time(0) };

// cannot assign Duration where Time is expected
func m4() { let bad4 : Time = Duration(0) };

// cannot use Int arithmetic directly on Time
func m5() { let bad5 = Time(1) + Time(2) };

// cannot unwrap an Int (unwrap only exists on newtypes)
func m6() { let bad6 = (42 : Int).unwrap };
