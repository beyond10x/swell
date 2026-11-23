// sec-runtime-semantics-methoddefinitionevaluation: a generator method is a generator template of kind method, defined as a method of its object under its key or its computed key, and starts as every generator does
({ *g(a) { yield a; }, *["h"]() {} });
