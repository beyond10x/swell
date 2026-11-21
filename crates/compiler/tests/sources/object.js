// sec-object-initializer: data properties in order, accessors, a numeric key as its string, __proto__ sets the prototype
({
  a: 1,
  "b": c,
  2: d,
  get e() { return 1; },
  set e(v) {},
  __proto__: null,
  f: function () {}
});
