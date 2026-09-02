import M "m";
module {
  public type Event = { #added : Nat; #removed : Nat };
  public type OldState = { events : [Event]; total : Nat };
  public type Registry = M.Map<Text, Event>;
}
