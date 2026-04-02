module {
    // Introduce fields x and y
    public func migration(_ : {}) : { x : Int; var y : Int } { { x = 0; var y = 0 } };
};
