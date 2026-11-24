// sec-async-arrow-function-definitions: SyntaxError "a rest parameter must be the last parameter"
var f = async (...a,) => 1;
