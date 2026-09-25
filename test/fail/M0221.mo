import { type T; type Result } "M0221/lib";

func _unableToDetermineType() {
  // Only able to infer type fields for val paths at the moment
  let { type K } = module { type K = Nat };
};

func _requiresAnnotation() {
  class HashMap() {
    public type Hash = Nat64;
  };
  // let map : HashMap = HashMap(); // Compiles fine
  let map = HashMap();
  let { type Hash } = map;
  let _x : Hash = 10;
};
