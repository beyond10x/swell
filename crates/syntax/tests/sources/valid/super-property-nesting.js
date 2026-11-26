// sec-super-keyword: parses
var o = {
  m(a = super.x, b = { [super.y]: 1 }) {
    function f() { return 1; }
    var g = function () { return { n() { return super.n; } }; };
    var h = x => x;
    var k = (y) => { return y; };
    var c = (super.x);
    var d = { [super.z]() { return 2; }, get [super.w]() { return 3; } };
    return c + super.x + f() + g() + h(1) + k(2) + typeof super.v + new super.C() + super.super + super.if;
  },
  set s(v = super.s) { super.s = v; }
};
