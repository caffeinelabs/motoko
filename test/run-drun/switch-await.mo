// test cps conversion of switch (bug fix)
actor a {
  public shared func go_ta() : async Text {
    switch 1 {
      case 1 { (await async "One") }
      case 2 { "Two" }
      case _ { "Other" }
    }
  };

  public shared func go_tt() : async Text {
    switch 1 {
      case 1 { "One" }
      case 2 { "Two" }
      case _ { "Other" }
    }
  };

  public shared func go_at() : async Text {
    switch (await async 1) {
      case 1 { "One" }
      case 2 { "Two" }
      case _ { "Other" }
    }
  };

  public shared func go_aa() : async Text {
    switch (await async 1) {
      case 1 { (await async "One") }
      case 2 { "Two" }
      case _ { "Other" }
    }
  };
};
ignore a.go_tt(); //OR-CALL ingress go_tt "DIDL\x00\x00"
ignore a.go_ta(); //OR-CALL ingress go_ta "DIDL\x00\x00"
ignore a.go_at(); //OR-CALL ingress go_at "DIDL\x00\x00"
ignore a.go_aa(); //OR-CALL ingress go_aa "DIDL\x00\x00"
