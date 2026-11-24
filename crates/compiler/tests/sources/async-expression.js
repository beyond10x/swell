// sec-runtime-semantics-instantiateasyncfunctionexpression: a named async function expression, its own name bound, and an await inside a call
var g = async function named(x) {
  named(await x);
};
