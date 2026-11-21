// sec-performeval: normal "undefined 1 undefined"
var r = eval("'use strict'; var y = 1; function h() {} y");
typeof y + " " + r + " " + typeof h;
