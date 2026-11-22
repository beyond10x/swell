// sec-switch-statement-runtime-semantics-evaluation: normal "ReferenceError,1,2"
var names = [];
function pick(x) {
  switch (x) {
    case 0:
      let a = 1;
      return a;
    default:
      return a;
  }
}
try {
  pick(1);
  names.push("none");
} catch (e) {
  names.push(e.name);
}
names.push(pick(0));
switch (2) {
  case 2:
    let b = 2;
    var get = function () { return b; };
}
names.push(get());
names.join(",");
