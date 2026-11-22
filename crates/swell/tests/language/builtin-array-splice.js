// sec-array.prototype.splice: normal "b,c|a,x,y,z,d|true"
// Two deleted, three inserted: the rest moves up; the deleted elements are a new Array.
var a = ["a", "b", "c", "d"];
var d = a.splice(1, 2, "x", "y", "z");
d.join() + "|" + a.join() + "|" + Array.isArray(d);
