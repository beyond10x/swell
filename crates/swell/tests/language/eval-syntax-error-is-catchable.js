// sec-performeval: normal true
var caught = false;
try { eval("var;"); } catch (e) { caught = e instanceof SyntaxError; }
caught;
