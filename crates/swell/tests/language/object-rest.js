// sec-destructuring-binding-patterns-runtime-semantics-restbindinginitialization: normal true
// A rest property collects the own enumerable properties the pattern did not read, in a
// declaration and in an assignment.
var { a, ["b"]: b, ...rest } = { a: 1, b: 2, c: 3, d: 4 };
var more;
({ c: a, ...more } = { c: 5, e: 6 });
a === 5 && b === 2 && Object.keys(rest).join() === "c,d" && Object.keys(more).join() === "e";
