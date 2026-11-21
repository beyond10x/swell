// sec-createdynamicfunction: normal "anonymous undefined"
var f = Function("return typeof anonymous;");
f.name + " " + f();
