// sec-runtime-semantics-restdestructuringassignmentevaluation: with a rest property each key read stays on the stack, a static one pushed as a String, until the rest object is made without them
var a, b, r, k, o;
({ a, [k]: b, ...r } = o);
function f(o) { var { [k]: c, x: y } = o; let { e, ...g } = o; return c + y + e + g; }
