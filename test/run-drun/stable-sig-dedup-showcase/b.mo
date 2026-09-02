import M "m";
module {
  public type Event = { #added : Nat; #removed : Nat };
  public type NewState = { events : [Event]; total : Nat };
  public type Registry = M.Map<Text, Event>;
}
