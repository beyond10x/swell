// sec-array-exotic-objects: normal true
var a = [1, 2, 3];
a[5] = 1;
var grown = a.length === 6;
a.length = 1;
grown && a.length === 1 && a[1] === undefined;
