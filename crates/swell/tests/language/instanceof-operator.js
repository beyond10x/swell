// sec-instanceofoperator: normal true
function F() {}
var o = new F();
o instanceof F && !({} instanceof F) && new RangeError("x") instanceof Error;
