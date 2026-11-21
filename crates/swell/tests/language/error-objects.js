// sec-error.prototype.tostring: normal "Error: m,true"
var caught;
try { null.x; } catch (e) { caught = e instanceof TypeError; }
"" + new Error("m") + "," + caught;
