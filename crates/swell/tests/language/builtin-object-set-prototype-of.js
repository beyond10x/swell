// sec-object.setprototypeof: normal "true,true,true,true,true,true,1,true,true"
// Object.setPrototypeOf sets [[Prototype]] and refuses a cycle, the immutable prototype of Object.prototype and an object that is not extensible with a TypeError.
var a = {}, b = {};
var r = [];
r.push(Object.setPrototypeOf(a, b) === a, Object.getPrototypeOf(a) === b);
try { Object.setPrototypeOf(b, a); } catch (e) { r.push(e instanceof TypeError); }
try { Object.setPrototypeOf(Object.prototype, {}); } catch (e) { r.push(e instanceof TypeError); }
r.push(Object.setPrototypeOf(Object.prototype, null) === Object.prototype);
try { Object.setPrototypeOf(Object.preventExtensions({}), b); } catch (e) { r.push(e instanceof TypeError); }
r.push(Object.setPrototypeOf(1, null));
try { Object.setPrototypeOf(undefined, null); } catch (e) { r.push(e instanceof TypeError); }
try { Object.setPrototypeOf({}, 1); } catch (e) { r.push(e instanceof TypeError); }
r.join();
