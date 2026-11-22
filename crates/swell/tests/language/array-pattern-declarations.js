// sec-destructuring-binding-patterns: normal "1,3;4,5;6;2,3;ab;9;10"
// An array binding pattern in var, let and const, with elisions, defaults, a nested pattern, a
// rest element, a parameter and a catch parameter.
var [a, , b] = [1, 2, 3];
let [c, [d] = [5]] = [4];
const [e = 6, ...rest] = [undefined, 2, 3];
function joined([x, y], ...[z]) { return x + y + z; }
var caught;
try { throw [9]; } catch ([first]) { caught = first; }
var [[deep] = [10]] = [];
a + "," + b + ";" + c + "," + d + ";" + e + ";" + rest.join() + ";" + joined("ab", "") + ";" + caught + ";" + deep;
