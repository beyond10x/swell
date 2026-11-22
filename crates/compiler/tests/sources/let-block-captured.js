// sec-blockdeclarationinstantiation: a block binding a closure captures is a slot of the record the block makes and leaves
{ let a = 1; var f = function () { return a; }; }
