// sec-blockdeclarationinstantiation: a block function a function refers to (here itself) is a slot of the record the block makes and leaves
{ function f() { return f; } f(); }
