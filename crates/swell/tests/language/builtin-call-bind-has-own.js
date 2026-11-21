// sec-function.prototype.bind: normal "truefalse"
// The bound `call` takes its this, hasOwnProperty, from the binding (sec-function.prototype.call).
// The global `Function` is not there yet, so `call` is read from a function.
var hasOwn = Object.prototype.hasOwnProperty;
var has = hasOwn.call.bind(hasOwn);
var o = { a: 1 };
"" + has(o, "a") + has(o, "toString");
