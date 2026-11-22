// sec-createperiterationenvironment: normal "0,1,2|3,3,3|a,b|1,3,5"
var lets = [], vars = [], keys = [], steps = [];
for (let i = 0; i < 3; i++) { lets.push(function () { return i; }); }
for (var j = 0; j < 3; j++) { vars.push(function () { return j; }); }
for (let k in { a: 1, b: 2 }) { keys.push(function () { return k; }); }
for (let n = 0; n < 6; n++) { steps.push(function () { return n; }); n++; }
function values(list) {
  var out = [];
  for (var m = 0; m < list.length; m++) {
    out.push(list[m]());
  }
  return out.join(",");
}
values(lets) + "|" + values(vars) + "|" + values(keys) + "|" + values(steps);
