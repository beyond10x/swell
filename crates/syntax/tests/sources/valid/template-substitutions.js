// sec-template-literals: parses
// Each substitution is an Expression with `in` allowed, also in the head of a `for`.
x = `${a}`;
x = `a${b}c${d}e`;
x = `${a, b}`;
x = `${a ? b : c}`;
for (var i = `${"a" in o}`; i; ) break;
x = `${/re/g}`;
x = `${{ a: 1 }.a}`;
x = `${function () { return 1; }()}`;
x = `${
  a
}`;
