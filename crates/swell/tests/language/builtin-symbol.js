// sec-symbol-description: normal "symbol,false,false,x,true,TypeError,0,Symbol,false,false,false,true"
// Symbol ( [ description ] ): a new Symbol at each call, whose [[Description]] is ToString of
// the argument or undefined; new Symbol() throws a TypeError (step 1). The constructor's
// "prototype" is not writable, not enumerable and not configurable (sec-symbol.prototype).
var a = Symbol("x"), b = Symbol("x"), r = [typeof a, a === b, Symbol() === Symbol(), a.description];
r.push(Symbol().description === undefined);
try { new Symbol(); } catch (e) { r.push(e.name); }
var p = Object.getOwnPropertyDescriptor(Symbol, "prototype");
r.push(Symbol.length, Symbol.name, p.writable, p.enumerable, p.configurable,
  Object.getPrototypeOf(Symbol) === Function.prototype);
r.join();
