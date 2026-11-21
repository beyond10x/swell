// sec-function-calls: parses
f();
f(1, 2);
f()();
a.b(c)(d).e[f]();
eval("1");
f(function () {}, [1], { a: 1 });
