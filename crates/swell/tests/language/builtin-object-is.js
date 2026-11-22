// sec-object.is: normal "true,false,true,true,false,true,false,2"
// Object.is is SameValue (sec-samevalue): NaN is itself, +0 and -0 differ.
[Object.is(NaN, NaN), Object.is(0, -0), Object.is(-0, -0), Object.is("a", "a"), Object.is({}, {}),
 Object.is(), Object.is(undefined, null), Object.is.length].join();
