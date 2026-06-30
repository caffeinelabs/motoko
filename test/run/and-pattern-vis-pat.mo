//MOC-FLAG -A=M0145
// Exercises vis_pat / vis_pat_field by placing public let bindings with
// structurally-varied patterns inside an object block.  pub_fields' calls
// vis_dec for each Public LetD, which recurses through the pattern via vis_pat.
// All arms except VarP were previously dark.
//
// The static check (which forbids non-trivial patterns in *libraries*) does
// not apply to the main program, so we can freely use complex patterns here.

let M = object {
  // TupP: vis_pat recurses into each element
  public let (a, b) = (1, 2);

  // ObjP + ValPF: vis_pat -> ObjP -> vis_pat_field ValPF -> vis_pat VarP
  public let { x = c } = { x = 3 };

  // OptP: vis_pat -> OptP pat1 -> vis_pat (VarP d)
  // `(?d)` is `ParP (OptP (VarP d))`: ParP and OptP both exercised
  public let (?d) = ?(4 : Nat);

  // TagP: vis_pat -> TagP (_, pat1) -> vis_pat (VarP e)
  // The type must be a single-constructor variant to be irrefutable
  public let (#tag e) = (#tag 5 : {#tag : Nat});

  // AnnotP: vis_pat -> AnnotP pat1 -> vis_pat inner (VarP f)
  public let (f : Nat) = 6;

  // AndP: vis_pat -> AndP -> vis_pat pat1 (AnnotP) + vis_pat pat2 (VarP)
  public let ((g : Nat) and h) = 7 : Nat;
};

assert (M.a + M.b + M.c + M.d + M.e + M.f + M.g + M.h == 35);
