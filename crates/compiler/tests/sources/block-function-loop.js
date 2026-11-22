// sec-blockdeclarationinstantiation: each entry of a loop body makes the function of the block anew
for (var i = 0; i < 2; i++) { f(); function f() {} }
