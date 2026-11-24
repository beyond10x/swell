// sec-asyncgenerator-definitions-propertydefinitionevaluation: an async generator method is an async generator template of kind method, defined as a method of its object under its key or its computed key, and starts, awaits, yields and returns as every async generator does
({ async *g(a) { yield await a; }, async *["h"]() {} });
