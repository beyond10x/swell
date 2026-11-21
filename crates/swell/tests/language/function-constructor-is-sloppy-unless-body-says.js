// sec-createdynamicfunction: normal "object undefined"
var sloppy = Function("return typeof this;");
var strict = Function("'use strict'; return typeof this;");
sloppy() + " " + strict();
