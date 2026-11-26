// sec-super-keyword: parses
var o = {
  m() { return super.x + super["y"]; },
  get g() { return super.g; },
  set s(v) { super.s = v; },
  *gen() { yield super.x; },
  async a() { return await super.x; },
  async *ag(p = super.p) { yield super[p]; },
  ["c"]() { super.x++; --super["y"]; super.z += 1; super.w ??= 1; delete super.x; return super.f(); },
  n() { return { m() { return super.x; } }; },
  t() { return super.tag`x${super.y}`; },
  q() { [super.a, { b: super.b }] = [1, { b: 2 }]; for (super.k in {}); return new super.C(); },
  r() { return super.f?.(super.x?.y); }
};
