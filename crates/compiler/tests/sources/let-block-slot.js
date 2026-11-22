// sec-blockdeclarationinstantiation: a block binding no closure captures is a local slot of the frame, read unchecked after its declaration
{ let a = 1; a; }
