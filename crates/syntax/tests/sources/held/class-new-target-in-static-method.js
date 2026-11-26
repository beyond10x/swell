// sec-class-definitions: held "class"
class A { static m() { return new.target; } static *g() { yield new.target; } }
