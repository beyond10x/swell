// sec-iteratorclose: normal "break:return;outer:return;return:return;throw:return,caught;done:"
// IteratorClose calls the iterator's return once on a break, on a continue to an outer loop, on
// a return and on a throw from the body, and not when the iterator is done.
function iterable(log) {
  var o = {};
  o[Symbol.iterator] = function () {
    var i = 0;
    return {
      next: function () { i++; return { value: i, done: i > 3 }; },
      "return": function () { log.push("return"); return {}; }
    };
  };
  return o;
}
var results = [], log;
log = []; for (var x of iterable(log)) break; results.push("break:" + log.join());
log = []; outer: for (var k = 0; k < 1; k++) { for (var x of iterable(log)) continue outer; }
results.push("outer:" + log.join());
log = []; (function () { for (var x of iterable(log)) return x; })();
results.push("return:" + log.join());
log = []; try { for (var x of iterable(log)) throw x; } catch (e) { log.push("caught"); }
results.push("throw:" + log.join());
log = []; for (var x of iterable(log)) ; results.push("done:" + log.join());
results.join(";");
