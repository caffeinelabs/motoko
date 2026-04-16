module {
    // Introduce fields x and y
    public func migration(_ : {}) : {
      x : Int; var y : Int; untyped : ()
    }
    { { x = 0;
        var y = 0;
	untyped = ()} };
};
