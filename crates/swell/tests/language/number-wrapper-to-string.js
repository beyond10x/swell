// sec-number.prototype.tostring: normal "255|ff|1|-0.1|z"
String(new Number(255)) + "|" + (255).toString(16) + "|" + new Number(1).toString() + "|" + (-0.5).toString(2) + "|" + new Number(35).toString(36);
