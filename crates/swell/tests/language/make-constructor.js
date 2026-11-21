// sec-makeconstructor: normal true
function F() {}
F.prototype.constructor === F && typeof F.prototype === "object";
