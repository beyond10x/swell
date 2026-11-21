// sec-ecmascript-function-objects-construct-argumentslist-newtarget: normal "2,1"
function F() { this.a = 1; return { a: 2 }; }
function G() { this.a = 1; return 5; }
new F().a + "," + new G().a;
