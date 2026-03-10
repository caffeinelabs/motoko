//MOC-FLAG -A=M0194
let _ = (actor this {}) : Any;
let _ = (actor {}) : Any;


let _ = (object this {}) : Any;
let _ = (object {}) : Any;


let _ = (actor this { public func x() : () { this.x() } }) : Any;
let _ = (actor { public func x() : () { x() } }) : Any;


let _ = (object this { public func x() { this.x() } }) : Any;
let _ = (object { public func x() { x() } }) : Any;

//SKIP comp
