// sec-class-definitions: held "class", "class field", "private method", "private field", "private name in", "class static block"
class C {
  a = 1;
  static b;
  ['c'] = 2
  static get = 3
  static
  e
  async
  w = super.constructor;
  get;
  #m() { return this.#x; }
  get #g() { return 1; }
  set #g(v) {}
  static async *#s() {}
  #x = 4;
  static #y;
  static #z = () => C.#y;
  has(o) { return #x in o && #x in o.p; }
  read(o) { return o?.#x + o?.p.#x + this.#m() + C.#y; }
  write(o) { o.#x = 1; [this.#x] = [2]; ({ x: this.#x } = {}); this.#x++; for (this.#x of []); }
  static {
    var v = new.target;
    super.x;
    function f() { return arguments; }
  }
  static {}
  f = class { #x; m() { return this.#x; } static { this.#x; } };
}
