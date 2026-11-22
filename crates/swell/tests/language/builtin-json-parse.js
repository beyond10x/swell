// sec-json.parse: normal "1,b,a|true|-Infinity|{"a":[1,{"b":null},[]]}|2|SyntaxError"
var o = JSON.parse('{"b":1,"a":2,"1":3}');
var keys = Object.getOwnPropertyNames(o).join();
var arr = JSON.parse(' [1, {"b": null}, []]\n');
var refused;
try { JSON.parse("[1,]"); refused = "parsed"; } catch (e) { refused = e.name; }
[keys, Array.isArray(arr), 1 / JSON.parse("-0"), JSON.stringify({ a: arr }), JSON.parse('{"x":1,"x":2}').x, refused].join("|");
