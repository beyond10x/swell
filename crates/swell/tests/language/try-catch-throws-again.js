// sec-runtime-semantics-catchclauseevaluation: throw URIError
try { throw 1; } catch (e) { throw new URIError("u"); }
