// sec-array.prototype.slice: normal "1,2|3|0,1,2,3,x"
// Then sec-array.prototype.concat: slice copies from start to before end, a negative index counting from the end; concat spreads an Array argument and appends any other value.
var a = [0, 1, 2, 3];
var s = a.slice(1, -1);
[s.join(), a.slice(-1).join(), a.concat("x").join()].join("|");
