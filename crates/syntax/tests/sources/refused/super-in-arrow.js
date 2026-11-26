// sec-super-keyword: refused "super in an arrow function is not supported before slice 8"
var o = { m() { return () => super.x; } };
