// sec-object.fromentries: normal "a=1,b=2|true|TypeError"
// Object.fromEntries makes an ordinary object from the entries of an iterable, each entry's 0
// the key and its 1 the value (sec-add-entries-from-iterable); an entry that is no object
// throws a TypeError.
var o = Object.fromEntries([["a", 1], ["b", 2]]);
var keys = Object.keys(o), out = [];
for (var i = 0; i < keys.length; i++) out.push(keys[i] + "=" + o[keys[i]]);
var name;
try { Object.fromEntries([1]); } catch (e) { name = e.name; }
[out.join(), Object.getPrototypeOf(o) === Object.prototype, name].join("|");
