// sec-class-definitions: held "class"
class A {
  constructor(a, b = 1, ...c) { this.a = a; }
  method() { return 1; }
  static staticMethod() {}
  get value() { return 1; }
  set value(v) {}
  static get staticValue() { return 2; }
  static set staticValue(v) {}
  *generator() { yield 1; }
  static *staticGenerator() { yield* a; }
  async asyncMethod() { await 1; }
  static async staticAsync() {}
  async *asyncGenerator() { yield await 1; }
  static async *staticAsyncGenerator() {}
  "string key"() {}
  'single quoted'() {}
  42() {}
  0x10() {}
  [computed + "key"]() {}
  static [computed]() {}
  get [computed]() { return 3; }
  ;;
  static() {}
  get() {}
  set() {}
  async() {}
  static
  prototypeless() {}
  static constructor() {}
  [class { [inner]() {} }.name]() {}
}
var B = class {};
var C = class Named { m() { return Named; } };
{ class D {} }
switch (0) { case 0: class E {} }
