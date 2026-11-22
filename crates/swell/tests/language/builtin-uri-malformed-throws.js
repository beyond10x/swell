// sec-decode: throw URIError
// A malformed escape, an octet sequence that is not UTF-8 and a lone surrogate throw a URIError (sec-decode, sec-encode); the last throw is the outcome only when every one before it threw.
var bad = ["%", "%4", "%G0", "%C3", "%C3%28", "%C0%80", "%ED%A0%80", "%F4%90%80%80"];
var caught = 0;
for (var i = 0; i < bad.length; i++) {
  try { decodeURI(bad[i]); } catch (e) { if (e instanceof URIError) caught++; }
  try { decodeURIComponent(bad[i]); } catch (e) { if (e instanceof URIError) caught++; }
}
try { encodeURIComponent("\ud800"); } catch (e) { if (e instanceof URIError) caught++; }
if (caught === 17) encodeURI("a\udc00");
