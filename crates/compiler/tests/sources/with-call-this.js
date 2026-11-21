// sec-evaluatecall: a call of a name looked up by name takes the with object as its this value
function g(o) { with (o) { f(); } }
