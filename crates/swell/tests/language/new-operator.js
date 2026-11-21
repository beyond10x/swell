// sec-new-operator: normal 4
function P(x) { this.x = x; }
P.prototype.get = function () { return this.x; };
new P(4).get();
