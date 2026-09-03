import A "a";
import M "m";
module {
  public type Nat = A.MyNat;
  public type Event = { #added : Nat; #removed : Nat };
  public type NewState = { events : [Event]; total : Nat };
  public type Registry = M.Map<Text, Event>;
  public type Tag = Text;
}
