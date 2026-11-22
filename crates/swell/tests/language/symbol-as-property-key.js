// sec-topropertykey: normal "1,2,true,true,false,a,a,3,false,true,true,false"
// A Symbol is a property key as it is: written and read by a member expression and by
// Object.defineProperty, found by `in` and hasOwnProperty, left out of Object.keys,
// Object.getOwnPropertyNames and for-in, and deleted. A Symbol object is a key through
// ToPrimitive, whose %Symbol.toPrimitive% answers its Symbol. A Symbol key is written with
// a member expression here, never as a computed property name.
var s = Symbol("k"), t = Symbol("k"), o = {};
o[s] = 1;
o[t] = 2;
var r = [o[s], o[t], s in o, o.hasOwnProperty(s), "k" in o];
o.a = 0;
var names = [];
for (var n in o) names.push(n);
r.push(Object.keys(o).join(), names.join());
Object.defineProperty(o, s, { value: 3, writable: false });
r.push(o[Object(s)]);
try { o[s] = 4; } catch (e) {}
r.push(o[s] === 4, delete o[t], Object.getOwnPropertyNames(o).length === 1, t in o);
r.join();
