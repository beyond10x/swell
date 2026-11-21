// sec-isstrictlyequal: normal "true,false"
var a = {};
var b = a;
(a === b) + "," + (a === {});
