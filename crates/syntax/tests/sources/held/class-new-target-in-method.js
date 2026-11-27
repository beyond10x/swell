// sec-class-definitions: held "class"
class A { m() { return new.target; } get g() { return new.target; } set s(v = new.target) {} }
