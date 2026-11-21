// sec-object-initializer: parses
x = {};
x = { a: 1, "b": 2, 3: 3, 0x4: 4, 1.5: 5, };
x = { if: 1, class: 2, function: 3, get: 4, set: 5, new: 6 };
x = { get a() { return 1; }, set a(v) {} };
x = { get "b"() { return 1; }, set 7(v) {}, get if() {} };
x = { __proto__: null, "a": 1 };
