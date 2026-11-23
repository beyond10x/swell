// sec-generator-objects: normal "a:1,2,3,true|b:sent|c:7,true,,true|d:caught x,9|e:TypeError"
// next runs a generator to each yield and answers { value, done } (sec-generator.prototype.next);
// the value given to next is the value of the yield; return completes it with the value given
// (sec-generator.prototype.return); throw throws at the yield, where a catch around it catches
// (sec-generator.prototype.throw); throw given to a completed generator throws the value.
function* count() { yield 1; yield 2; return 3; }
var g = count(), out = [];
for (var r = g.next(); !r.done; r = g.next()) out.push(r.value);
out.push(r.value, g.next().done);
var a = "a:" + out.join();
function* echo() { var x = yield "first"; return x; }
var e = echo(); e.next();
var b = "b:" + e.next("sent").value;
function* stop() { yield 1; yield 2; }
var s = stop(); s.next();
var ret = s.return(7), after = s.next();
var c = "c:" + [ret.value, ret.done, after.value, after.done].join();
function* guard() { try { yield 1; } catch (x) { yield "caught " + x; } return 9; }
var t = guard(); t.next();
var d = "d:" + t.throw("x").value + "," + t.next().value;
var u = count(); u.return();
var thrown;
try { u.throw(new TypeError("t")); } catch (x) { thrown = x.name; }
[a, b, c, d, "e:" + thrown].join("|");
