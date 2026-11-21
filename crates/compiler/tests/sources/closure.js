// sec-environment-records: a binding an inner function uses lives in an environment, the others in slots of the frame
function outer(a, b) {
  var c, d;
  try {} catch (e) { return function () { return b + d + e; }; }
  return a + c;
}
