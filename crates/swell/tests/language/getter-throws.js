// sec-ordinaryget: throw EvalError
var o = { get x() { throw new EvalError("g"); } };
o.x;
