// Test masked br_table dispatch for variant switches (4+ arms)

type Color = { #red; #green; #blue; #yellow };

func colorName(c : Color) : Text =
  switch c {
    case (#red)    "red";
    case (#green)  "green";
    case (#blue)   "blue";
    case (#yellow) "yellow";
  };

assert (colorName (#red)    == "red");
assert (colorName (#green)  == "green");
assert (colorName (#blue)   == "blue");
assert (colorName (#yellow) == "yellow");

type Weekday = { #Mon; #Tue; #Wed; #Thu; #Fri; #Sat; #Sun };

func isWeekend(d : Weekday) : Bool =
  switch d {
    case (#Mon) false;
    case (#Tue) false;
    case (#Wed) false;
    case (#Thu) false;
    case (#Fri) false;
    case (#Sat) true;
    case (#Sun) true;
  };

assert (not (isWeekend (#Mon)));
assert (not (isWeekend (#Fri)));
assert (isWeekend (#Sat));
assert (isWeekend (#Sun));

// Same dispatch expressed with or-patterns — exercises same-body arm merging
func isWeekendOr(d : Weekday) : Bool =
  switch d {
    case (#Mon or #Tue or #Wed or #Thu or #Fri) false;
    case (#Sat or #Sun) true;
  };

assert (not (isWeekendOr (#Mon)));
assert (not (isWeekendOr (#Fri)));
assert (isWeekendOr (#Sat));
assert (isWeekendOr (#Sun));

// Variant with payload — sub-pattern binding still works
type Shape = { #circle : Float; #rect : (Float, Float); #tri : Float; #dot : () };

func area(s : Shape) : Float =
  switch s {
    case (#circle r)   r * r * 3.14159;
    case (#rect(w, h)) w * h;
    case (#tri b)      b * b * 0.5;
    case (#dot _)      0.0;
  };

assert (area (#dot ()) == 0.0);
assert (area (#rect(3.0, 4.0)) == 12.0);
