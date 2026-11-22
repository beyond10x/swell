// sec-declarative-environment-records-getbindingvalue-n-s: normal "ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,1"
var names = [];
function check(f) {
  try {
    f();
    names.push("none");
  } catch (e) {
    names.push(e.name);
  }
}
check(function () { x; let x = 1; });
check(function () { x = 2; let x; });
check(function () { typeof x; let x; });
check(function () { x++; let x = 0; });
check(function () { let x = x; });
check(function () { function g() { return y; } g(); let y = 1; });
check(function () { for (var i = 0; i < 2; i++) { if (i === 1) { w; } let w = i; } });
check(function () { for (let k in k) {} });
function late() {
  let r = 1;
  return r;
}
names.push(late());
names.join(",");
