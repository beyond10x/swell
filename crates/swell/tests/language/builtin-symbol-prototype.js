// sec-properties-of-the-symbol-prototype-object: normal "Symbol(d),Symbol(),true,true,d,true,false,true,Symbol,[object Symbol],[Symbol.toPrimitive],1,false,true,TypeError,TypeError"
// %Symbol.prototype%: toString answers SymbolDescriptiveString (sec-symboldescriptivestring),
// valueOf and [Symbol.toPrimitive] answer thisSymbolValue (sec-thissymbolvalue), the
// description getter its [[Description]] (sec-symbol.prototype.description), and
// [Symbol.toStringTag] is "Symbol"; each reads a Symbol or a Symbol object, and throws a
// TypeError for anything else.
var s = Symbol("d"), o = Object(s), r = [];
r.push(s.toString(), Symbol().toString(), s.valueOf() === s, o.valueOf() === s, o.description);
var desc = Object.getOwnPropertyDescriptor(Symbol.prototype, "description");
r.push(typeof desc.get === "function", desc.set !== undefined, desc.configurable);
r.push(Symbol.prototype[Symbol.toStringTag], Object.prototype.toString.call(o));
var tp = Object.getOwnPropertyDescriptor(Symbol.prototype, Symbol.toPrimitive);
r.push(tp.value.name, tp.value.length, tp.writable, tp.configurable);
try { Symbol.prototype.valueOf.call({}); } catch (e) { r.push(e.name); }
try { desc.get.call("d"); } catch (e) { r.push(e.name); }
r.join();
