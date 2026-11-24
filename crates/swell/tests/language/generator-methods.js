// sec-runtime-semantics-methoddefinitionevaluation: normal "1,2,3;g,[k];true"
// A generator method of an object literal is a generator function: its calls make generator
// objects that run its body to each yield, it is named by its key, computed or not, it is an
// enumerable property of its object, and it has no [[Construct]].
var key = "k";
var o = {
  *g(n) { for (var i = 1; i <= n; i++) yield i; },
  *[key + ""]() {},
};
var values = [];
for (var v of o.g(3)) values.push(v);
var refused = false;
try { new o.g(1); } catch (e) { refused = e instanceof TypeError; }
[values.join(), [o.g.name, "[" + o.k.name + "]"].join(), refused && Object.keys(o).length === 2].join(";");
