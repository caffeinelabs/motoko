// this migration should not be applied to any imported class, only the main prog
module {

  public func migration({}) : { f : {#f} } = {
   f = #f
  };

}
