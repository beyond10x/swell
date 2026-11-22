// sec-let-and-const-declarations-runtime-semantics-evaluation: normal true
// A let or const pattern binds block-scoped names; a const name is not writable, and the names
// are uninitialized until the pattern binds them.
let { a, b: c } = { a: 1, b: 2 };
const { d = 4 } = {};
var thrown = [];
try { d = 5; } catch (e) { thrown.push(e.name); }
{
  let { a } = { a: 10 };
  thrown.push(a);
}
try { let { x = y, y } = {}; } catch (e) { thrown.push(e.name); }
var closures = [];
for (let { k } = { k: 0 }; k < 2; k++) {
  closures.push(function () { return k; });
}
a === 1 && c === 2 && d === 4 && closures[0]() === 0 && closures[1]() === 1
  && thrown.join() === "TypeError,10,ReferenceError";
