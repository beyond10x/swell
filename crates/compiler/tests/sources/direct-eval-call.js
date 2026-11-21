// sec-function-calls-runtime-semantics-evaluation: a call of the name eval, in parentheses or not, is a CallEval, and every name of a script that holds one is looked up by name; a member call, a comma expression, an optional call and new of it are ordinary
eval("1");
(eval)("2", 3);
o.eval("4");
(0, eval)("5");
eval?.("6");
new eval("7");
