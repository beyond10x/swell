// sec-array-initializer: parses
var a = [...b];
var c = [0, ...d, , ...e, 1, ...[...f], ];
var g = [, ...h, , ];
var i = [...j = k, ...l ? m : n, ...function () {}, ...(0, o)];
