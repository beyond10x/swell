// sec-object-initializer-static-semantics-early-errors: SyntaxError "duplicate __proto__ property in an object literal"
x = { __proto__: null, "__proto__": null };
