// sec-createdynamicfunction: normal true
var f = Function("a", "b, c", "return a + b + c");
f(1, 2, 3) === 6 && f.length === 3 && new Function("return 1")() === 1 &&
  Function()() === undefined && Object.getPrototypeOf(f) === Function.prototype;
