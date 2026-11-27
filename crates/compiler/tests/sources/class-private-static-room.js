// sec-runtime-semantics-classdefinitionevaluation: the static private methods and an accessor pair of one class are added to F in source order, each add with the room of the elements of F's level, the pair counted once
class C { static #a() {} static get #b() { return 1; } static set #b(v) {} static #c() {} static t() { return C.#a(); } }
