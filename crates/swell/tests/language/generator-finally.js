// sec-generatorresumeabrupt: normal "inner,outer;5,true;boom,inner,outer;2,false,9,true,after;true"
// return given to a generator suspended inside try blocks runs the finally blocks around the
// yield, innermost first, and completes it with the value given; a throw given runs them too and
// leaves the generator as a throw; a finally that yields stops the return there, and the next
// next ends the return with its value; a throw that leaves the body completes the generator.
var log = [];
function* nested() {
  try { try { yield 1; } finally { log.push("inner"); } } finally { log.push("outer"); }
}
var g = nested(); g.next();
var r = g.return(5);
var a = log.join() + ";" + r.value + "," + r.done;
log = [];
var h = nested(); h.next();
var caught;
try { h.throw("boom"); } catch (x) { caught = x; }
var b = caught + "," + log.join();
log = [];
function* hold() { try { yield 1; } finally { yield 2; log.push("after"); } }
var k = hold(); k.next();
var y = k.return(9), z = k.next();
var c = [y.value, y.done, z.value, z.done, log.join()].join();
[a, b, c, h.next().done].join(";");
