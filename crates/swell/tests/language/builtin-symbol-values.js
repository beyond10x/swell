// sec-toprimitive: normal "trim,true,false,[object Tagged],{"a":1},true,false,true,TypeError"
// Builtins of slice 3 meet Symbol values and symbol keys: the `this` of a String method is
// converted through its %Symbol.toPrimitive% with the hint "string"; `instanceof` calls
// %Symbol.hasInstance% of its right side and Function.prototype[%Symbol.hasInstance%] answers
// OrdinaryHasInstance; Object.prototype.toString reads %Symbol.toStringTag%; JSON.stringify
// leaves out symbol keys and Symbol values; Object.hasOwn takes a Symbol key; ToString of a
// Symbol throws a TypeError. Written without a computed property name, a spread or `for`-`of`.
var hints = [];
var holder = {};
holder[Symbol.toPrimitive] = function (hint) { hints.push(hint); return "  trim  "; };
var r = [String.prototype.trim.call(holder) + (hints.join() === "string" ? "" : hints.join())];
function F() {}
var always = {};
Object.defineProperty(always, Symbol.hasInstance, { value: function () { return 1; } });
r.push({} instanceof always, Function.prototype[Symbol.hasInstance].call(F, {}));
var tagged = {};
tagged[Symbol.toStringTag] = "Tagged";
r.push(Object.prototype.toString.call(tagged));
var json = { a: 1, b: Symbol("b") };
json[Symbol("c")] = 2;
r.push(JSON.stringify(json));
var key = Symbol("own");
var owner = {};
owner[key] = 0;
r.push(Object.hasOwn(owner, key), Object.hasOwn({}, key),
  Function.prototype[Symbol.hasInstance].call(F, new F()));
try { "" + Symbol(); r.push("answered"); } catch (e) { r.push(e.name); }
r.join();
