// sec-boolean.prototype.valueof: normal "2|true|true|taken"
var f = new Boolean(false);
(new Boolean(true) + 1) + "|" + String(new Boolean(true)) + "|" + (f == false) + "|" + (f ? "taken" : "not taken");
