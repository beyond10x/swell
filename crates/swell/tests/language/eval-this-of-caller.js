// sec-performeval: normal true
var o = { m: function () { return eval("this"); } };
o.m() === o && eval("this") === this;
