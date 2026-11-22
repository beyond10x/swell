// sec-runtime-semantics-propertydefinitionevaluation: normal true
// A computed key is evaluated and converted by ToPropertyKey before its value, in source order.
var log = [];
var i = 0;
var o = {
  [(log.push("k1"), "a" + i++)]: (log.push("v1"), 1),
  [(log.push("k2"), "a" + i++)]: (log.push("v2"), 2),
  [{ toString: function () { log.push("k3"); return "b"; } }]: (log.push("v3"), 3)
};
log.join() === "k1,v1,k2,v2,k3,v3" && o.a0 === 1 && o.a1 === 2 && o.b === 3;
