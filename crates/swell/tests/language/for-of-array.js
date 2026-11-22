// sec-for-in-and-for-of-statements: normal "1,2,3;30;4"
// ForIn/OfHeadEvaluation (sec-runtime-semantics-forinofheadevaluation) gets the iterator of an
// Array, ForIn/OfBodyEvaluation steps its elements in order and a script's completion value is
// the last value of the body; an element pushed while the loop runs is seen, since the Array
// Iterator reads the length at every step.
var out = [], a = [1, 2, 3];
for (var x of a) out.push(x);
var c = eval("for (var y of a) y * 10;");
var grown = [1], n = 0;
for (var z of grown) { n++; if (grown.length < 4) grown.push(z + 1); }
out.join() + ";" + c + ";" + n;
