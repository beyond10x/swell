// sec-symbol.for: normal "true,false,true,k,true,true,TypeError,false,true"
// Symbol.for ( key ) answers the Symbol of the GlobalSymbolRegistry for ToString(key), made the
// first time; Symbol.keyFor ( symbol ) answers that key (sec-keyforsymbol), undefined for a
// Symbol Symbol.for did not make, and throws a TypeError for a value that is not a Symbol.
// The registry's descriptors: both functions are methods of length 1.
var k = Symbol.for("k");
var r = [k === Symbol.for("k"), k === Symbol("k"), Symbol.for(1) === Symbol.for("1")];
r.push(Symbol.keyFor(k), Symbol.keyFor(Symbol("k")) === undefined,
  Symbol.keyFor(Symbol.toStringTag) === undefined);
try { Symbol.keyFor("k"); } catch (e) { r.push(e.name); }
var d = Object.getOwnPropertyDescriptor(Symbol, "for");
r.push(d.enumerable, Symbol.for.length === 1 && Symbol.keyFor.length === 1 && d.writable);
r.join();
