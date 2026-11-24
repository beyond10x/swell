// sec-runtime-semantics-methoddefinitionevaluation: an async method is an async template of kind method, defined as a method of its object under its key or its computed key, and starts, awaits and returns as every async function does
({ async m(a) { return await a; }, async ["n"]() {} });
