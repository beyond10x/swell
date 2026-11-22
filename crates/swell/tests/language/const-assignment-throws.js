// sec-declarative-environment-records-setmutablebinding-n-v-s: normal "TypeError,TypeError,TypeError,TypeError,TypeError,ReferenceError,1,true"
var names = [];
function check(f) {
  try {
    f();
    names.push("none");
  } catch (e) {
    names.push(e.name);
  }
}
const c = 1;
check(function () { c = 2; });
check(function () { const d = 1; d += 1; });
check(function () { const d = 1; function g() { d = 2; } g(); });
check(function () { const d = 1; d++; });
check(function () { for (const i = 0; i < 1; i++) {} });
check(function () { d = 0; const d = 1; });
names.push(c);
var message;
try { c = 3; } catch (e) { message = e.message; }
names.push(message === "Assignment to the constant binding " + "c");
names.join(",");
